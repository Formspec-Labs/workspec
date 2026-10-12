// Tests use the maintained permissive in-memory router, not deployment grants.
use super::*;
use stack_common_http::tenant::{HeaderConfig, TenantScope};
use trellis_service_client::{
    AppendActor, ComputeContext, SubstrateClient, TrellisServiceClient, TrellisServiceClientConfig,
    wos_provenance_append_request,
};

// Abort on assertion failure as well as normal return; never leave a test listener.
struct FixtureServer(tokio::task::JoinHandle<()>);
impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn native_custody_key_passes_real_router_and_replays_without_another_event() {
    let mut kernel_json =
        serde_json::to_value(kernel_with_actors("1.0.0", serde_json::json!([]))).unwrap();
    kernel_json["lifecycle"]["states"] = serde_json::json!({
        "Draft": { "type": "atomic", "transitions": [{ "event": "close", "target": "Closed" }] },
        "Closed": { "type": "atomic" }
    });
    let kernel = serde_json::from_value(kernel_json).unwrap();
    let mut runtime = runtime_with_kernel(kernel);
    let process_id = stack_common_typeid::mint_process_id();
    let created = runtime
        .create_process(CreateProcessRequest {
            process_id: process_id.clone(),
            tenant: None,
            definition_url: "urn:test:populator".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: None,
        })
        .expect("actual native process creation");
    runtime
        .enqueue_event(
            &process_id,
            PendingEvent {
                origin: None,
                event: "close".into(),
                actor_id: None,
                data: None,
                timestamp: String::new(),
                idempotency_token: None,
            },
        )
        .expect("enqueue actual lifecycle event");
    runtime
        .drain_until_idle(&process_id)
        .expect("execute actual native transition");
    let provenance = runtime
        .load_provenance_window(&process_id, 0, usize::MAX)
        .unwrap();
    // Initialization emits unregistered kinds too. Select the actual registered
    // transition by its original log position; do not rewrite or fabricate it.
    let cursor = provenance
        .iter()
        .position(|record| record.record_kind == wos_events::ProvenanceKind::StateTransition)
        .expect("native state-transition provenance");
    assert_eq!(provenance[cursor].from_state.as_deref(), Some("Draft"));
    assert_eq!(provenance[cursor].to_state.as_deref(), Some("Closed"));
    let context = crate::CustodyAppendContext {
        event_type_prefix: "wos.kernel".into(),
        case_id: Some(created.case_ledger_id.clone()),
        max_inline_record_bytes: None,
        workflow_ref: Some(created.definition_url.clone()),
    };
    let input = runtime
        .load_custody_append_window(&process_id, cursor, 1, context.clone())
        .expect("actual runtime custody record")
        .remove(0);
    assert!(stack_common_typeid::is_case_ledger_id(&input.case_id));
    assert!(stack_common_typeid::is_valid_record_type_id(
        &input.record_id
    ));
    assert!(
        format!(
            "wos-runtime:provenance:{}:{}",
            input.case_id, input.record_id
        )
        .len()
            > trellis_types::IDEMPOTENCY_KEY_MAX_LEN
    );
    let original: ProvenanceRecord =
        serde_json::from_value(input.record_json_view().unwrap()).unwrap();
    let tenant = TenantScope {
        tenant: "tenant-a".into(),
        workspace: "workspace-a".into(),
        environment: "test".into(),
        cell: "cell-a".into(),
    };
    let request = wos_provenance_append_request(
        &input.case_id,
        tenant.clone(),
        stack_common_hash::sha256_nul_separated_parts(&[
            "wos-runtime:provenance",
            &input.case_id,
            &input.record_id,
        ]),
        AppendActor::service("wos-runtime"),
        original.clone(),
        ComputeContext::no_delegated_compute("wos-runtime"),
    )
    .unwrap();
    assert!(trellis_types::idempotency_key_length_in_bound(
        request.idempotency_key.as_bytes()
    ));

    let app = trellis_server::router(trellis_server::test_harness::multi_producer_memory_state())
        .expect("maintained Trellis router");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let _server = FixtureServer(tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    }));
    let client = TrellisServiceClient::new(
        TrellisServiceClientConfig::new(format!("http://{address}"), HeaderConfig::wos())
            .with_timeout(std::time::Duration::from_secs(10)),
    )
    .unwrap();
    let appender = TrellisCustodyAppender::new(
        &client,
        tenant.clone(),
        AppendActor::service("wos-runtime"),
        ComputeContext::no_delegated_compute("wos-runtime"),
    );
    let outcomes = appender
        .append_window(&mut runtime, &process_id, cursor, 1, context.clone())
        .await
        .expect("actual HTTP/router/Core append");
    assert_eq!(outcomes.len(), 1);
    let first = &outcomes[0];
    assert_eq!(first.case_id, input.case_id);
    assert_eq!(first.record_id, input.record_id);
    assert_eq!(first.sequence, 0);
    let digest = first.checkpoint_ref.rsplit('/').next().unwrap();
    let pinned_before = client
        .bundle(&input.case_id, digest, &tenant)
        .await
        .unwrap();
    let replay = client
        .append_event(request.clone())
        .await
        .expect("exact transport replay");
    assert_eq!(replay.event_id, first.event_id);
    assert_eq!(replay.sequence, first.sequence);
    assert_eq!(replay.canonical_event_hash, first.canonical_event_hash);
    assert_eq!(replay.checkpoint_ref, first.checkpoint_ref);
    assert_eq!(replay.bundle_ref, first.bundle_ref);
    let mut changed = request.clone();
    changed.payload["id"] = stack_common_typeid::mint_provenance_id().into();
    let conflict = client
        .append_event(changed)
        .await
        .expect_err("same key with changed body");
    assert!(conflict.to_string().contains("409"), "{conflict}");
    assert_eq!(
        client
            .bundle(&input.case_id, digest, &tenant)
            .await
            .unwrap(),
        pinned_before
    );
    assert_eq!(
        client.head_bundle(&input.case_id, &tenant).await.unwrap(),
        pinned_before
    );
    let entries = integrity_bundle::read_stored_zip(&pinned_before).unwrap();
    let events = entries
        .iter()
        .find(|entry| entry.path().rsplit('/').next() == Some(trellis_export_writer::EVENTS_MEMBER))
        .unwrap();
    let integrity_cbor::Value::Array(events) =
        integrity_cbor::decode_cbor_value(events.bytes()).unwrap()
    else {
        panic!("native event member must be an array")
    };
    assert_eq!(
        events.len(),
        1,
        "replay/conflict must not append another event"
    );
    let stored = runtime
        .load_provenance_window(&process_id, cursor, 1)
        .unwrap();
    assert_eq!(stored[0].id, original.id);
    assert_eq!(
        stored[0].canonical_event_hash.as_deref(),
        Some(first.canonical_event_hash.as_str())
    );
    assert!(
        appender
            .append_window(&mut runtime, &process_id, cursor, 1, context)
            .await
            .unwrap()
            .is_empty(),
        "stamped runtime record must be skipped"
    );
}

#[tokio::test]
async fn native_provenance_full_window_preserves_every_original_record_and_replays() {
    use integrity_cbor::{
        decode_cbor_value, encode_cbor_value, map_lookup_bytes, map_lookup_map, map_lookup_u64,
    };
    use wos_events::{NativeProvenanceEvent, WOS_NATIVE_PROVENANCE_EVENT_LITERAL};

    let kernel: KernelDocument = serde_json::from_value(serde_json::json!({
        "$wosWorkflow":"1.0", "url":"urn:test:complete-custody", "version":"1.0.0",
        "lifecycle":{"initialState":"Draft","states":{
            "Draft":{"type":"atomic", "onEntry":[
                {"action":"startTimer","timerId":"review-clock","duration":"PT2H","event":"review.timeout"},
                {"action":"createTask","taskRef":"review","assignTo":"reviewer","contractRef":"reviewForm",
                    "responseMappingRef":"urn:mapping:response","completionEvent":"review.completed","failureEvent":"review.failed"}
            ], "transitions":[{"event":"close","target":"Closed"}]},
            "Closed":{"type":"atomic"}
        }}, "contracts":{"reviewForm":{"binding":"formspec","ref":"urn:formspec:review","definitionVersion":"1.0.0"}}
    })).unwrap();
    let mut runtime = runtime_with_kernel(kernel);
    let process_id = stack_common_typeid::mint_process_id();
    let created = runtime
        .create_process(CreateProcessRequest {
            process_id: process_id.clone(),
            tenant: None,
            definition_url: "urn:test:complete-custody".into(),
            definition_version: "1.0.0".into(),
            initial_case_state: Some(serde_json::json!({"approved":false})),
        })
        .unwrap();
    runtime
        .enqueue_event(
            &process_id,
            PendingEvent {
                origin: None,
                event: "close".into(),
                actor_id: None,
                data: None,
                timestamp: String::new(),
                idempotency_token: None,
            },
        )
        .unwrap();
    runtime.drain_until_idle(&process_id).unwrap();
    let original = runtime
        .load_provenance_window(&process_id, 0, usize::MAX)
        .unwrap();
    assert_eq!(original[0].record_kind, wos_events::ProvenanceKind::OnEntry);
    for kind in [
        wos_events::ProvenanceKind::TimerCreated,
        wos_events::ProvenanceKind::TaskCreated,
        wos_events::ProvenanceKind::TaskPresented,
        wos_events::ProvenanceKind::StateTransition,
    ] {
        assert!(
            original.iter().any(|r| r.record_kind == kind),
            "missing native {kind:?}"
        );
    }
    let context = crate::CustodyAppendContext {
        event_type_prefix: "wos.kernel".into(),
        case_id: Some(created.case_ledger_id.clone()),
        max_inline_record_bytes: None,
        workflow_ref: Some(created.definition_url.clone()),
    };
    let inputs = runtime
        .load_custody_append_window(&process_id, 0, usize::MAX, context.clone())
        .unwrap();
    assert_eq!(
        inputs.len(),
        original.len(),
        "every original record belongs to the complete window"
    );
    let tenant = TenantScope {
        tenant: "tenant-a".into(),
        workspace: "workspace-a".into(),
        environment: "test".into(),
        cell: "cell-a".into(),
    };
    let actor = AppendActor::service("wos-runtime");
    let compute = ComputeContext::no_delegated_compute("wos-runtime");
    let first_native =
        NativeProvenanceEvent::from_record_dcbor_slice(inputs[0].record_bytes()).unwrap();
    assert_eq!(
        first_native.record(),
        &inputs[0].record_json_view().unwrap()
    );
    let first_request = trellis_service_client::SubstrateAppendRequest::new_json(
        inputs[0].case_id.clone(),
        tenant.clone(),
        WOS_NATIVE_PROVENANCE_EVENT_LITERAL,
        stack_common_hash::sha256_nul_separated_parts(&[
            "wos-runtime:provenance",
            &inputs[0].case_id,
            &inputs[0].record_id,
        ]),
        actor.clone(),
        first_native.payload().clone(),
        compute.clone(),
    )
    .unwrap();

    let app = trellis_server::router(trellis_server::test_harness::multi_producer_memory_state())
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let _server = FixtureServer(tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    }));
    let client = TrellisServiceClient::new(
        TrellisServiceClientConfig::new(format!("http://{address}"), HeaderConfig::wos())
            .with_timeout(std::time::Duration::from_secs(10)),
    )
    .unwrap();
    let appender = TrellisCustodyAppender::new(&client, tenant.clone(), actor, compute);
    let outcomes = appender
        .append_window(&mut runtime, &process_id, 0, usize::MAX, context.clone())
        .await
        .unwrap();
    assert_eq!(outcomes.len(), original.len());
    let head_before = client
        .head_bundle(&created.case_ledger_id, &tenant)
        .await
        .unwrap();
    let original_zip_hash = stack_common_hash::sha256_prefixed(&head_before);
    // This fixture checks the export's embedded signing registry. Independent
    // agency trust qualification remains outside this local router test.
    let report = trellis_verify_wos::verify_export_zip(&head_before);
    assert!(report.trellis.structure_verified, "{report:#?}");
    assert!(report.trellis.integrity_verified, "{report:#?}");
    assert!(report.relying_party_valid(), "{report:#?}");
    let entries = integrity_bundle::read_stored_zip(&head_before).unwrap();
    let member = entries
        .iter()
        .find(|e| e.path().rsplit('/').next() == Some(trellis_export_writer::EVENTS_MEMBER))
        .unwrap();
    let integrity_cbor::Value::Array(events) = decode_cbor_value(member.bytes()).unwrap() else {
        panic!("events array")
    };
    assert_eq!(
        events.len(),
        original.len(),
        "no original audit fact dropped"
    );
    for (position, ((event, source), input)) in
        events.iter().zip(&original).zip(&inputs).enumerate()
    {
        let signed = integrity_cose::decode_cose_sign1(&encode_cbor_value(event).unwrap()).unwrap();
        let canonical =
            decode_cbor_value(signed.payload().expect("embedded canonical event")).unwrap();
        let map = canonical.as_map().unwrap();
        assert_eq!(map_lookup_u64(map, "sequence").unwrap(), position as u64);
        let header = map_lookup_map(map, "header").unwrap();
        let event_type =
            String::from_utf8(map_lookup_bytes(header, "event_type").unwrap()).unwrap();
        assert_eq!(event_type, input.event_type);
        let payload_ref = map_lookup_map(map, "payload_ref").unwrap();
        let bytes = map_lookup_bytes(payload_ref, "ciphertext").unwrap();
        let actual = if event_type == WOS_NATIVE_PROVENANCE_EVENT_LITERAL {
            let native = NativeProvenanceEvent::from_dcbor_slice(&bytes).unwrap();
            assert_eq!(
                native.record(),
                &input.record_json_view().unwrap(),
                "entire original input JSON retained"
            );
            native.record().clone()
        } else {
            integrity_cbor::dcbor_bytes_to_json(&bytes).unwrap()
        };
        assert_eq!(
            actual,
            serde_json::to_value(source).unwrap(),
            "original record {position} retained in order"
        );
        assert_eq!(outcomes[position].record_id, source.id);
        assert_eq!(outcomes[position].sequence, position as u64);
    }
    // Deliberately tamper with an existing generic payload; preserve its
    // original COSE signature and every other archive member. The owning
    // verifier must refuse this altered evidence before relying on it.
    let mut tampered_events = events.clone();
    let position = inputs
        .iter()
        .position(|input| input.event_type == WOS_NATIVE_PROVENANCE_EVENT_LITERAL)
        .unwrap();
    let envelope = match &mut tampered_events[position] {
        integrity_cbor::Value::Tag(18, value) => value.as_mut(),
        value => value,
    };
    let integrity_cbor::Value::Array(parts) = envelope else {
        panic!("COSE array")
    };
    let integrity_cbor::Value::Bytes(canonical_bytes) = &mut parts[2] else {
        panic!("COSE payload bytes")
    };
    let mut altered = decode_cbor_value(canonical_bytes).unwrap();
    let payload_ref = native_test_cbor_field(&mut altered, "payload_ref");
    let integrity_cbor::Value::Bytes(ciphertext) =
        native_test_cbor_field(payload_ref, "ciphertext")
    else {
        panic!("ciphertext bytes")
    };
    let mut payload = decode_cbor_value(ciphertext).unwrap();
    let record = native_test_cbor_field(&mut payload, "record");
    *native_test_cbor_field(record, "recordKind") =
        integrity_cbor::Value::Text("unknownFutureKind".into());
    *ciphertext = encode_cbor_value(&payload).unwrap();
    *canonical_bytes = encode_cbor_value(&altered).unwrap();
    let changed_member = encode_cbor_value(&integrity_cbor::Value::Array(tampered_events)).unwrap();
    let mut tampered_bundle = integrity_bundle::Bundle::new();
    for entry in &entries {
        tampered_bundle.add_entry(if entry.path() == member.path() {
            integrity_bundle::BundleEntry::new(entry.path(), changed_member.clone())
        } else {
            entry.clone()
        });
    }
    let tampered_zip = tampered_bundle.to_zip_bytes().unwrap();
    let refused = trellis_verify_wos::verify_export_zip(&tampered_zip);
    assert!(!refused.relying_party_valid(), "{refused:#?}");
    assert!(!refused.trellis.integrity_verified, "{refused:#?}");
    assert_eq!(
        stack_common_hash::sha256_prefixed(&head_before),
        original_zip_hash
    );
    let stored = runtime
        .load_provenance_window(&process_id, 0, usize::MAX)
        .unwrap();
    for ((source, stamped), outcome) in original.iter().zip(&stored).zip(&outcomes) {
        let mut expected = serde_json::to_value(source).unwrap();
        expected["canonicalEventHash"] = outcome.canonical_event_hash.clone().into();
        assert_eq!(
            serde_json::to_value(stamped).unwrap(),
            expected,
            "receipt changes only original hash stamp"
        );
    }
    let replay = client.append_event(first_request.clone()).await.unwrap();
    assert_eq!(replay.event_id, outcomes[0].event_id);
    assert_eq!(
        replay.canonical_event_hash,
        outcomes[0].canonical_event_hash
    );
    assert_eq!(replay.checkpoint_ref, outcomes[0].checkpoint_ref);
    assert_eq!(replay.bundle_ref, outcomes[0].bundle_ref);
    let mut changed = first_request;
    changed.payload["record"]["data"] = serde_json::json!({"changed":true});
    assert!(
        client
            .append_event(changed)
            .await
            .unwrap_err()
            .to_string()
            .contains("409")
    );
    assert!(
        appender
            .append_window(&mut runtime, &process_id, 0, usize::MAX, context)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        client
            .head_bundle(&created.case_ledger_id, &tenant)
            .await
            .unwrap(),
        head_before,
        "transport replay, conflict and stamped retry do not alter complete history"
    );
}

fn native_test_cbor_field<'a>(
    value: &'a mut integrity_cbor::Value,
    name: &str,
) -> &'a mut integrity_cbor::Value {
    let integrity_cbor::Value::Map(fields) = value else {
        panic!("expected CBOR map")
    };
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == &integrity_cbor::Value::Text(name.into()))
        .expect("controlled fixture field")
        .1
}
