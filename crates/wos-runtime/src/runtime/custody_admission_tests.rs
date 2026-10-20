// Rust guideline compliant 2026-02-21

#[derive(Default)]
struct AdmissionHandler {
    calls: std::sync::Mutex<Vec<&'static str>>,
    outcomes: std::sync::Mutex<Vec<TrellisCustodyAppendOutcome>>,
    refuse: Option<TrellisCustodyAppendStage>,
    persist_fail_at: Option<usize>,
}

#[async_trait]
impl TrellisCustodyAppendHandler for AdmissionHandler {
    async fn authorize_before_append(
        &self,
        _input: &CustodyAppendInput,
        scope: &LedgerScope,
    ) -> Result<(), RuntimeError> {
        assert_eq!(scope, &test_ledger("original-ledger"));
        self.calls.lock().unwrap().push("before");
        if self.refuse == Some(TrellisCustodyAppendStage::BeforeAppend) {
            return Err(RuntimeError::Service("before refused".into()));
        }
        Ok(())
    }

    async fn authorize_after_append(
        &self,
        input: &CustodyAppendInput,
        outcome: &TrellisCustodyAppendOutcome,
        scope: &LedgerScope,
    ) -> Result<(), RuntimeError> {
        assert_eq!(scope, &test_ledger("original-ledger"));
        assert_eq!(input.case_id, outcome.case_id);
        assert_eq!(input.record_id, outcome.record_id);
        self.calls.lock().unwrap().push("after");
        if self.refuse == Some(TrellisCustodyAppendStage::AfterAppendAdmission) {
            return Err(RuntimeError::Service("after refused".into()));
        }
        Ok(())
    }

    async fn persist_acknowledged(
        &self,
        outcome: &TrellisCustodyAppendOutcome,
        scope: &LedgerScope,
    ) -> Result<(), RuntimeError> {
        assert_eq!(scope, &test_ledger("original-ledger"));
        self.calls.lock().unwrap().push("persist");
        // Model a real commit followed by a final authority refusal.
        self.outcomes.lock().unwrap().push(outcome.clone());
        if self.refuse == Some(TrellisCustodyAppendStage::Persistence)
            || self.persist_fail_at == Some(self.outcomes.lock().unwrap().len())
        {
            return Err(RuntimeError::Service("post-commit refusal".into()));
        }
        Ok(())
    }
}

/// An appender that routes every native case to the ledger `original-ledger`.
fn routed_appender(client: &dyn SubstrateClient) -> TrellisCustodyAppender<'_> {
    TrellisCustodyAppender::new(
        client,
        AppendActor::service("wos-runtime"),
        ComputeContext::no_delegated_compute("wos-runtime"),
        test_ledger("original-ledger"),
    )
}

#[tokio::test]
async fn custody_admission_routing_preserves_native_case_payload_and_key() {
    let context = test_context(fixture_minter().mint_case_ledger_id());
    let input = test_input(&context);
    let native_case = input.case_id.clone();
    let record_id = input.record_id.clone();
    let key = trellis_idempotency_key(&test_ledger("original-ledger"), &input);
    let expected_payload = wos_provenance_append_request(
        test_ledger(&native_case),
        key.clone(),
        AppendActor::service("wos-runtime"),
        provenance_record_from_input(&input).unwrap(),
        ComputeContext::no_delegated_compute("wos-runtime"),
    )
    .unwrap()
    .payload;
    let mut runtime = RecordingDurableRuntime::new(input);
    let client = RecordingSubstrateClient::default();
    let handler = AdmissionHandler::default();
    let outcomes = routed_appender(&client)
        .append_window_with_handler(&mut runtime, &handler, "process-1", 0, 25, context)
        .await
        .unwrap();
    assert_eq!(outcomes[0].case_id, native_case);
    assert_eq!(outcomes[0].record_id, record_id);
    assert_eq!(
        *handler.calls.lock().unwrap(),
        vec!["before", "after", "persist"]
    );
    assert_eq!(*handler.outcomes.lock().unwrap(), outcomes);
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests[0].scope, test_ledger("original-ledger"));
    assert_eq!(requests[0].idempotency_key, key);
    assert_eq!(requests[0].payload, expected_payload);
    assert_eq!(requests[0].event_type, outcomes[0].event_type);
    assert!(runtime.applied.lock().unwrap().is_empty());
}

#[tokio::test]
async fn custody_admission_failures_retain_truth_and_do_not_hash_stamp_or_resend() {
    for stage in [
        TrellisCustodyAppendStage::BeforeAppend,
        TrellisCustodyAppendStage::AfterAppendAdmission,
        TrellisCustodyAppendStage::Persistence,
    ] {
        let context = test_context(fixture_minter().mint_case_ledger_id());
        let input = test_input(&context);
        let record_id = input.record_id.clone();
        let mut runtime = RecordingDurableRuntime::new(input);
        let client = RecordingSubstrateClient::default();
        let handler = AdmissionHandler {
            refuse: Some(stage),
            ..Default::default()
        };
        let failure = routed_appender(&client)
            .append_window_with_handler(&mut runtime, &handler, "process-1", 0, 25, context)
            .await
            .unwrap_err();
        assert_eq!(failure.stage, stage);
        let published = stage != TrellisCustodyAppendStage::BeforeAppend;
        assert_eq!(
            client.requests.lock().unwrap().len(),
            usize::from(published)
        );
        assert_eq!(failure.acknowledged_outcome.is_some(), published);
        if let Some(actual) = failure.acknowledged_outcome {
            assert_eq!(actual.record_id, record_id);
            assert_eq!(
                actual.checkpoint_ref,
                format!(
                    "trellis://{}/checkpoints/cp_test",
                    test_ledger("original-ledger")
                )
            );
        }
        assert!(failure.persisted_outcomes.is_empty());
        assert!(runtime.applied.lock().unwrap().is_empty());
        assert_eq!(
            handler.outcomes.lock().unwrap().len(),
            usize::from(stage == TrellisCustodyAppendStage::Persistence)
        );
    }
}

struct AlteredResultClient {
    inner: RecordingSubstrateClient,
    alteration: &'static str,
}

#[async_trait]
impl SubstrateClient for AlteredResultClient {
    async fn append_event(
        &self,
        request: SubstrateAppendRequest,
    ) -> Result<SubstrateAppendResult, StackError> {
        let scope = request.scope.clone();
        let mut result = self.inner.append_event(request).await?;
        if self.alteration == "append" {
            return Err(StackError::unavailable("network outcome unknown"));
        }
        match self.alteration {
            "namespace" => {
                result.checkpoint_ref =
                    format!("trellis://{}/checkpoints/cp", test_ledger("another-ledger"))
            }
            "empty-checkpoint" => result.checkpoint_ref = format!("trellis://{scope}/checkpoints/"),

            "hash" => result.canonical_event_hash = "wrong".into(),
            "blank-event" => result.event_id.clear(),
            "blank-hash" => result.canonical_event_hash.clear(),
            "blank-checkpoint" => result.checkpoint_ref.clear(),
            "blank-prior-hash" => result.prior_event_hash = Some(String::new()),
            _ => unreachable!("known test alteration"),
        }
        Ok(result)
    }
    async fn head_bundle(&self, _scope: &LedgerScope) -> Result<Vec<u8>, StackError> {
        unreachable!("not used")
    }
    async fn bundle(
        &self,
        _scope: &LedgerScope,
        _checkpoint_digest: &str,
    ) -> Result<Vec<u8>, StackError> {
        unreachable!("not used")
    }
    async fn signing_key_registry(&self, _scope: &LedgerScope) -> Result<Vec<u8>, StackError> {
        unreachable!("not used")
    }
    async fn event_type_registry(
        &self,
        _scope: &LedgerScope,
    ) -> Result<serde_json::Value, StackError> {
        unreachable!("not used")
    }
}

#[tokio::test]
async fn custody_admission_invalid_actual_results_stop_before_post_admission_and_persist() {
    for alteration in [
        "namespace",
        "empty-checkpoint",
        "hash",
        "blank-event",
        "blank-hash",
        "blank-checkpoint",
        "blank-prior-hash",
    ] {
        let context = test_context(fixture_minter().mint_case_ledger_id());
        let mut runtime = RecordingDurableRuntime::new(test_input(&context));
        let client = AlteredResultClient {
            inner: Default::default(),
            alteration,
        };
        let appender = routed_appender(&client);
        let handler = AdmissionHandler::default();
        let failure = appender
            .append_window_with_handler(&mut runtime, &handler, "process-1", 0, 25, context)
            .await
            .unwrap_err();
        assert_eq!(failure.stage, TrellisCustodyAppendStage::ResultValidation);
        assert!(failure.acknowledged_outcome.is_none());
        assert_eq!(*handler.calls.lock().unwrap(), vec!["before"]);
        assert_eq!(client.inner.requests.lock().unwrap().len(), 1);
        assert!(runtime.applied.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn custody_admission_stamped_legacy_record_skips_without_invented_association() {
    let context = test_context(fixture_minter().mint_case_ledger_id());
    let case_id = context.case_id.as_deref().unwrap();
    let mut record = ProvenanceRecord::state_transition(
        &fixture_minter(),
        "open",
        "closed",
        "close",
        Some("actor"),
    );
    record.canonical_event_hash = Some(TEST_HASH.into());
    let metadata = context
        .metadata_for_provenance_record(case_id, 0, &record)
        .unwrap();
    let input = CustodyAppendInput::from_provenance_record(&record, &context, metadata).unwrap();
    let mut runtime = RecordingDurableRuntime::new(input);
    let client = RecordingSubstrateClient::default();
    let handler = AdmissionHandler::default();
    let outcomes = routed_appender(&client)
        .append_window_with_handler(&mut runtime, &handler, "process-1", 0, 25, context)
        .await
        .unwrap();
    assert!(outcomes.is_empty());
    assert!(handler.calls.lock().unwrap().is_empty());
    assert!(client.requests.lock().unwrap().is_empty());
    assert!(runtime.applied.lock().unwrap().is_empty());
}

#[tokio::test]
async fn custody_admission_partial_window_retains_confirmed_and_uncertain_results() {
    let context = test_context(fixture_minter().mint_case_ledger_id());
    let first = test_input(&context);
    let second = test_input(&context);
    let first_id = first.record_id.clone();
    let second_id = second.record_id.clone();
    let mut runtime = RecordingDurableRuntime::new(first);
    runtime.inputs.push(second);
    let client = RecordingSubstrateClient::default();
    let handler = AdmissionHandler {
        persist_fail_at: Some(2),
        ..Default::default()
    };
    let failure = routed_appender(&client)
        .append_window_with_handler(&mut runtime, &handler, "process-1", 0, 25, context)
        .await
        .unwrap_err();
    assert_eq!(failure.stage, TrellisCustodyAppendStage::Persistence);
    assert_eq!(failure.persisted_outcomes.len(), 1);
    assert_eq!(failure.persisted_outcomes[0].record_id, first_id);
    assert_eq!(failure.acknowledged_outcome.unwrap().record_id, second_id);
    assert_eq!(handler.outcomes.lock().unwrap().len(), 2);
    assert_eq!(client.requests.lock().unwrap().len(), 2);
    assert!(runtime.applied.lock().unwrap().is_empty());
}

#[tokio::test]
async fn custody_admission_network_error_keeps_external_status_unknown() {
    let context = test_context(fixture_minter().mint_case_ledger_id());
    let mut runtime = RecordingDurableRuntime::new(test_input(&context));
    let client = AlteredResultClient {
        inner: Default::default(),
        alteration: "append",
    };
    let appender = routed_appender(&client);
    let handler = AdmissionHandler::default();
    let failure = appender
        .append_window_with_handler(&mut runtime, &handler, "process-1", 0, 25, context)
        .await
        .unwrap_err();
    assert_eq!(failure.stage, TrellisCustodyAppendStage::Append);
    assert!(failure.acknowledged_outcome.is_none());
    assert_eq!(*handler.calls.lock().unwrap(), vec!["before"]);
    assert_eq!(client.inner.requests.lock().unwrap().len(), 1);
    assert!(runtime.applied.lock().unwrap().is_empty());
}

#[tokio::test]
async fn custody_admission_generic_native_bytes_share_existing_publisher() {
    let context = test_context(fixture_minter().mint_case_ledger_id());
    let case_id = context.case_id.as_deref().unwrap().to_owned();
    let mut record = ProvenanceRecord::action_executed(&fixture_minter(), "open", "log");
    crate::stamp_provenance(std::slice::from_mut(&mut record), "2026-10-11T00:00:00Z");
    assert!(NativeProvenanceEvent::from_record(&record).is_err());
    let kernel = serde_json::from_value(serde_json::json!({
        "$wosWorkflow":"1.0", "version":"1.0.0",
        "lifecycle":{"initialState":"open","states":{"open":{"type":"atomic"}}}
    }))
    .unwrap();
    // Apply the maintained metadata step used before native persistence.
    // Timestamp-only stamping does not produce a complete custody record.
    crate::runtime::provenance::populate_provenance_record_fields(
        std::slice::from_mut(&mut record),
        &kernel,
        "1.0.0",
    );
    assert!(record.record_kind.canonical_event_literal().is_none());
    let metadata = context
        .metadata_for_provenance_record(&case_id, 0, &record)
        .unwrap();
    let input = CustodyAppendInput::from_provenance_record(&record, &context, metadata).unwrap();
    assert_eq!(input.event_type, WOS_NATIVE_PROVENANCE_EVENT_LITERAL);
    let original = NativeProvenanceEvent::from_record_dcbor_slice(input.record_bytes()).unwrap();
    let expected_payload = original.payload().clone();
    let expected_key = trellis_idempotency_key(&test_ledger("original-ledger"), &input);
    let mut runtime = RecordingDurableRuntime::new(input);
    let client = RecordingSubstrateClient::default();
    let handler = AdmissionHandler::default();
    let outcome = routed_appender(&client)
        .append_window_with_handler(&mut runtime, &handler, "process-1", 0, 25, context)
        .await
        .unwrap();
    assert_eq!(outcome[0].case_id, case_id);
    let requests = client.requests.lock().unwrap();
    assert_eq!(requests[0].scope, test_ledger("original-ledger"));
    assert_eq!(requests[0].event_type, WOS_NATIVE_PROVENANCE_EVENT_LITERAL);
    assert_eq!(requests[0].payload, expected_payload);
    assert_eq!(requests[0].idempotency_key, expected_key);
    assert!(runtime.applied.lock().unwrap().is_empty());
}
