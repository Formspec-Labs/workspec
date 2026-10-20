// Rust guideline compliant 2026-02-21
// Included within runtime::tests so these acceptance tests use maintained fixtures.
mod deferred_callback_acceptance {
    use super::*;
    use wos_core::instance::PendingEvent;

    fn kernel(version: &str) -> KernelDocument {
        serde_json::from_value(serde_json::json!({
            "$wosWorkflow":"1.0","url":"urn:test:deferred-callback","version":version,
            "lifecycle":{"initialState":"open","states":{
                "open":{"type":"atomic","onEntry":[{"action":"invokeService","serviceRef":"notice","callbackPhase":"outbound"}],
                    "transitions":[{"event":{"kind":"message","name":"receipt"},"target":"open","actions":[
                        {"action":"invokeService","serviceRef":"notice","callbackPhase":"inbound"}]}]}
            }}
        })).unwrap()
    }

    fn profile() -> IntegrationProfileDocument {
        serde_json::from_value(serde_json::json!({
            "targetWorkflow":{"url":"urn:test:deferred-callback"},"bindings":{"notice":{
                "type":"callback","dispatchMode":"deferred","callbackEventType":"com.example.receipt",
                "eventType":"com.example.request","dataMapping":{"reference":"caseFile.reference"},
                "outputBinding":{"caseFile.receipt":"$"}
            }}
        })).unwrap()
    }

    fn fixture() -> (WosRuntime, String, Arc<AtomicUsize>) {
        fixture_with_profile(profile())
    }

    fn fixture_with_profile(
        profile: IntegrationProfileDocument,
    ) -> (WosRuntime, String, Arc<AtomicUsize>) {
        let mut runtime = runtime_with_kernels(vec![kernel("1.0.0"), kernel("2.0.0")])
            .with_integration_profile(profile);
        let service = RecordingService::with_response(serde_json::Value::Null);
        let calls = service.calls.clone();
        runtime.service = Box::new(service);
        let id = fixture_minter().mint_process_id();
        runtime
            .create_process(CreateProcessRequest {
                process_id: id.clone(),
                tenant: None,
                definition_url: "urn:test:deferred-callback".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: Some(serde_json::json!({"reference":"synthetic"})),
            })
            .unwrap();
        (runtime, id, calls)
    }

    fn receipt(record: &RuntimeRecord) -> serde_json::Value {
        let subject = record.process.pending_callbacks.keys().next().unwrap();
        serde_json::json!({"id":"receipt-own-identity","source":"urn:test:receipt","specversion":"1.0",
            "type":"com.example.receipt","subject":subject,"data":{"qualifying":true,"sentAt":"2024-03-09T16:00:00Z"}})
    }

    fn enqueue(runtime: &mut WosRuntime, id: &str, data: serde_json::Value) {
        runtime
            .enqueue_event(
                id,
                PendingEvent {
                    origin: None,
                    event: "receipt".into(),
                    actor_id: None,
                    data: Some(data),
                    timestamp: "2024-03-09T16:00:00Z".into(),
                    idempotency_token: None,
                },
            )
            .unwrap();
    }

    #[test]
    fn deferred_outbound_persists_native_request_without_provider_call() {
        let (runtime, id, calls) = fixture();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let record = runtime.store.load_record(&id).unwrap();
        let wire = serde_json::to_value(&record.process).unwrap();
        assert_eq!(wire["pendingOutboundEvents"].as_object().unwrap().len(), 1);
        assert_eq!(record.process.pending_callbacks.len(), 1);
        let mut restored = record.clone();
        restored.process = serde_json::from_value(wire.clone()).unwrap();
        let mut store = InMemoryStore::new();
        store.restore_record(restored).unwrap();
        assert_eq!(
            serde_json::to_value(&store.load_record(&id).unwrap().process).unwrap(),
            wire
        );
    }

    #[test]
    fn deferred_receipt_uses_own_identity_and_consumes_pair_after_mapping() {
        let (mut runtime, id, calls) = fixture();
        let before = runtime.store.load_record(&id).unwrap();
        enqueue(&mut runtime, &id, receipt(&before));
        runtime.drain_once(&id).unwrap();
        let after = runtime.store.load_record(&id).unwrap();
        assert_eq!(after.process.case_state["receipt"]["qualifying"], true);
        // The self transition creates a NEW outbound request on entry, not reuse.
        assert_eq!(after.process.pending_callbacks.len(), 1);
        assert_ne!(
            after.process.pending_callbacks.keys().next(),
            before.process.pending_callbacks.keys().next()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(
            after
                .provenance_log
                .iter()
                .any(|p| p.record_kind == ProvenanceKind::CallbackReceived)
        );
    }

    #[test]
    fn malformed_unknown_wrong_type_and_drift_receipts_have_no_effects() {
        for mode in ["missing-id", "unknown", "wrong-type", "same-id", "drift"] {
            let (mut runtime, id, _) = fixture();
            let original = runtime.store.load_record(&id).unwrap();
            let mut incoming = receipt(&original);
            match mode {
                "missing-id" => {
                    incoming.as_object_mut().unwrap().remove("id");
                }
                "unknown" => incoming["subject"] = serde_json::json!("unknown"),
                "wrong-type" => incoming["type"] = serde_json::json!("com.example.foreign"),
                "same-id" => {
                    incoming["id"] = serde_json::json!(
                        original
                            .process
                            .pending_callbacks
                            .values()
                            .next()
                            .unwrap()
                            .invocation_id
                    )
                }
                _ => runtime
                    .integration_profile
                    .as_mut()
                    .unwrap()
                    .bindings
                    .get_mut("notice")
                    .unwrap()
                    .output_binding
                    .insert("caseFile.foreign".into(), "$".into())
                    .map(|_| ())
                    .unwrap_or(()),
            }
            enqueue(&mut runtime, &id, incoming);
            let before =
                serde_json::to_value(&runtime.store.load_record(&id).unwrap().process).unwrap();
            assert!(runtime.drain_once(&id).is_err(), "{mode}");
            assert_eq!(
                serde_json::to_value(&runtime.store.load_record(&id).unwrap().process).unwrap(),
                before,
                "{mode}"
            );
        }
    }

    #[test]
    fn deferred_pair_request_and_snapshot_cannot_change_or_disappear_on_update() {
        let (mut runtime, id, _) = fixture();
        let original = runtime.store.load_record(&id).unwrap();
        for field in ["request", "bindingSnapshot", "erased"] {
            let mut changed = original.clone();
            let mut wire = serde_json::to_value(&changed.process).unwrap();
            let pair = wire["pendingOutboundEvents"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .next()
                .unwrap();
            match field {
                "request" => pair["request"]["data"] = serde_json::json!({"tampered":true}),
                "bindingSnapshot" => {
                    pair["bindingSnapshot"]["description"] = serde_json::json!("changed")
                }
                _ => {
                    wire.as_object_mut()
                        .unwrap()
                        .remove("pendingOutboundEvents");
                    wire["pendingCallbacks"] = serde_json::json!({});
                }
            }
            changed.process = serde_json::from_value(wire).unwrap();
            assert!(runtime.store.save_record(changed).is_err(), "{field}");
            assert_eq!(
                serde_json::to_value(&runtime.store.load_record(&id).unwrap().process).unwrap(),
                serde_json::to_value(&original.process).unwrap()
            );
        }
    }

    #[test]
    fn unresolved_deferred_request_refuses_definition_migration() {
        let (mut runtime, id, _) = fixture();
        let before =
            serde_json::to_value(&runtime.store.load_record(&id).unwrap().process).unwrap();
        assert!(matches!(
            runtime.migrate(&id, "2.0.0", MigrationMap::default(), None),
            Err(RuntimeError::MigrationRejected(_))
        ));
        assert_eq!(
            serde_json::to_value(&runtime.store.load_record(&id).unwrap().process).unwrap(),
            before
        );
    }

    #[test]
    fn deferred_capacity_and_duplicate_subject_refuse_without_identity_allocation() {
        for mode in ["count", "bytes", "duplicate"] {
            let (mut runtime, id, calls) = fixture();
            let mut record = runtime.store.load_record(&id).unwrap();
            let original = serde_json::to_value(&record.process).unwrap();
            let mut binding = profile().bindings.remove("notice").unwrap();
            if mode == "count" {
                runtime = runtime.with_deferred_callback_limits(0, 1024 * 1024);
            }
            if mode == "bytes" {
                runtime = runtime.with_deferred_callback_limits(1000, 0);
            }
            if mode == "duplicate" {
                binding.extensions.insert(
                    "subject".into(),
                    serde_json::json!(record.process.pending_callbacks.keys().next().unwrap()),
                );
            }
            let observed=wos_core::eval::ObservedAction {
                provenance_record_id: None,
                transition_record_id: None,
                action:serde_json::from_value(serde_json::json!({"action":"invokeService","serviceRef":"notice","callbackPhase":"outbound"})).unwrap(),
                actor_id:None,lifecycle_state:"open".into(),event_data:None,task_expiry_timer:None,
            };
            assert!(
                runtime
                    .invoke_integration_binding(
                        &mut record,
                        &kernel("1.0.0"),
                        &observed,
                        "notice",
                        &binding,
                        "2024-03-09T16:00:00Z"
                    )
                    .is_err(),
                "{mode}"
            );
            assert_eq!(
                serde_json::to_value(&record.process).unwrap(),
                original,
                "{mode}"
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn explicit_outbound_does_not_reclassify_initiating_cloud_event_as_receipt() {
        let (mut runtime, id, calls) = fixture();
        let mut record = runtime.store.load_record(&id).unwrap();
        let observed=wos_core::eval::ObservedAction {
            provenance_record_id: None,
            transition_record_id: None,
            action:serde_json::from_value(serde_json::json!({"action":"invokeService","serviceRef":"notice","callbackPhase":"outbound"})).unwrap(),
            actor_id:None,lifecycle_state:"open".into(),event_data:Some(receipt(&record)),task_expiry_timer:None,
        };
        let binding = profile().bindings.remove("notice").unwrap();
        runtime
            .invoke_integration_binding(
                &mut record,
                &kernel("1.0.0"),
                &observed,
                "notice",
                &binding,
                "2024-03-09T16:00:00Z",
            )
            .unwrap();
        assert_eq!(record.process.pending_outbound_events.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(record.process.case_state["receipt"].is_null());
    }

    #[test]
    fn deferred_restore_refuses_orphan_and_malformed_native_requests() {
        let (runtime, id, _) = fixture();
        let original = runtime.store.load_record(&id).unwrap();
        for mode in ["orphan", "malformed", "sequence", "migration"] {
            let mut invalid = original.clone();
            match mode {
                "orphan" => invalid.process.pending_callbacks.clear(),
                "malformed" => {
                    invalid
                        .process
                        .pending_outbound_events
                        .values_mut()
                        .next()
                        .unwrap()
                        .request["specversion"] = serde_json::json!("0.3")
                }
                "sequence" => invalid.process.next_deferred_callback_sequence = 0,
                _ => invalid.process.definition_version = "2.0.0".into(),
            }
            assert!(matches!(
                InMemoryStore::new().restore_record(invalid),
                Err(StoreError::InvalidIntegrationEvidence(_))
            ));
        }
    }

    #[test]
    fn deferred_save_failure_retries_original_pair_and_deterministic_next_identity() {
        let (mut runtime, id, calls) = fixture();
        let original = runtime.store.load_record(&id).unwrap();
        let next_id = format!(
            "{id}:notice:deferred:{}",
            original.process.next_deferred_callback_sequence
        );
        let mut failing = FailingStore::new(2);
        failing.inner.restore_record(original.clone()).unwrap();
        runtime.store = Box::new(failing);
        enqueue(&mut runtime, &id, receipt(&original));
        let before =
            serde_json::to_value(&runtime.store.load_record(&id).unwrap().process).unwrap();
        assert!(runtime.drain_once(&id).is_err());
        assert_eq!(
            serde_json::to_value(&runtime.store.load_record(&id).unwrap().process).unwrap(),
            before
        );
        runtime.drain_once(&id).unwrap();
        assert_eq!(
            runtime
                .store
                .load_record(&id)
                .unwrap()
                .process
                .pending_outbound_events
                .values()
                .next()
                .unwrap()
                .invocation_id,
            next_id
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn absent_dispatch_preserves_immediate_callbacks_and_old_wire_shape() {
        let mut p = serde_json::to_value(profile()).unwrap();
        p["bindings"]["notice"]
            .as_object_mut()
            .unwrap()
            .remove("dispatchMode");
        let mut k = serde_json::to_value(kernel("1.0.0")).unwrap();
        k["lifecycle"]["states"]["open"]["onEntry"][0]
            .as_object_mut()
            .unwrap()
            .remove("callbackPhase");
        let mut runtime = runtime_with_kernel(serde_json::from_value(k).unwrap())
            .with_integration_profile(serde_json::from_value(p).unwrap());
        let service = RecordingService::with_response(serde_json::Value::Null);
        let calls = service.calls.clone();
        runtime.service = Box::new(service);
        let id = fixture_minter().mint_process_id();
        let process = runtime
            .create_process(CreateProcessRequest {
                process_id: id,
                tenant: None,
                definition_url: "urn:test:deferred-callback".into(),
                definition_version: "1.0.0".into(),
                initial_case_state: Some(serde_json::json!({"reference":"synthetic"})),
            })
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(process.pending_callbacks.len(), 1);
        let wire = serde_json::to_value(process).unwrap();
        assert!(wire.get("pendingOutboundEvents").is_none());
        assert!(wire.get("nextDeferredCallbackSequence").is_none());
    }

    #[test]
    fn deferred_dispatch_and_phase_fail_closed_on_unsupported_shapes() {
        assert!(
            serde_json::from_value::<crate::integration::IntegrationBinding>(
                serde_json::json!({"type":"request-response","dispatchMode":null})
            )
            .is_err()
        );
        assert!(serde_json::from_value::<wos_core::model::kernel::Action>(serde_json::json!({"action":"invokeService","serviceRef":"notice","callbackPhase":null})).is_err());
        for kind in ["request-response", "tool", "event-emit"] {
            assert!(
                serde_json::from_value::<crate::integration::IntegrationBinding>(
                    serde_json::json!({"type":kind,"dispatchMode":"immediate"})
                )
                .is_err()
            );
        }
        for expected in [
            serde_json::Value::Null,
            serde_json::json!(false),
            serde_json::json!(" "),
        ] {
            assert!(serde_json::from_value::<crate::integration::IntegrationBinding>(serde_json::json!({"type":"callback","dispatchMode":"deferred","callbackEventType":expected})).is_err());
        }
        let mut k = serde_json::to_value(kernel("1.0.0")).unwrap();
        k["lifecycle"]["states"]["open"]["onEntry"][0]
            .as_object_mut()
            .unwrap()
            .remove("callbackPhase");
        let mut runtime = runtime_with_kernel(serde_json::from_value(k).unwrap())
            .with_integration_profile(profile());
        let id = fixture_minter().mint_process_id();
        assert!(
            runtime
                .create_process(CreateProcessRequest {
                    process_id: id.clone(),
                    tenant: None,
                    definition_url: "urn:test:deferred-callback".into(),
                    definition_version: "1.0.0".into(),
                    initial_case_state: None
                })
                .is_err()
        );
        assert!(runtime.store.load_record(&id).is_err());
    }

    #[test]
    fn deferred_missing_output_path_refuses_without_partial_mapping_or_pair_removal() {
        let mut selected = profile();
        let binding = selected.bindings.get_mut("notice").unwrap();
        binding.output_binding = HashMap::from([
            ("caseFile.first".into(), "$.qualifying".into()),
            ("caseFile.last".into(), "$.missing".into()),
        ]);
        let (mut runtime, id, calls) = fixture_with_profile(selected);
        let original = runtime.store.load_record(&id).unwrap();
        enqueue(&mut runtime, &id, receipt(&original));
        let before = runtime.store.load_record(&id).unwrap();
        assert!(runtime.drain_once(&id).is_err());
        let after = runtime.store.load_record(&id).unwrap();
        assert_eq!(
            serde_json::to_value(&after.process).unwrap(),
            serde_json::to_value(&before.process).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&after.provenance_log).unwrap(),
            serde_json::to_value(&before.provenance_log).unwrap()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn program_constructed_dispatch_and_unreachable_unsupported_phase_refuse_before_provider() {
        let (mut runtime, id, calls) = fixture();
        let mut record = runtime.store.load_record(&id).unwrap();
        let original = serde_json::to_value(&record.process).unwrap();
        let mut invalid = profile().bindings.remove("notice").unwrap();
        invalid.kind = crate::integration::IntegrationBindingKind::RequestResponse;
        let observed=wos_core::eval::ObservedAction {
            provenance_record_id: None,
            transition_record_id: None,
            action:serde_json::from_value(serde_json::json!({"action":"invokeService","serviceRef":"notice","callbackPhase":"outbound"})).unwrap(),
            actor_id:None,lifecycle_state:"open".into(),event_data:None,task_expiry_timer:None,
        };
        assert!(
            runtime
                .invoke_integration_binding(
                    &mut record,
                    &kernel("1.0.0"),
                    &observed,
                    "notice",
                    &invalid,
                    "2024-03-09T16:00:00Z"
                )
                .is_err()
        );
        assert_eq!(serde_json::to_value(&record.process).unwrap(), original);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let mut value = serde_json::to_value(kernel("1.0.0")).unwrap();
        value["lifecycle"]["states"]["neverReached"] = serde_json::json!({"type":"atomic","onEntry":[{"action":"setData","path":"caseFile.invalid","value":true,"callbackPhase":"inbound"}]});
        let mut invalid_runtime = runtime_with_kernel(serde_json::from_value(value).unwrap())
            .with_integration_profile(profile());
        let new_id = fixture_minter().mint_process_id();
        assert!(
            invalid_runtime
                .create_process(CreateProcessRequest {
                    process_id: new_id.clone(),
                    tenant: None,
                    definition_url: "urn:test:deferred-callback".into(),
                    definition_version: "1.0.0".into(),
                    initial_case_state: None
                })
                .is_err()
        );
        assert!(invalid_runtime.store.load_record(&new_id).is_err());
    }

    #[test]
    fn deferred_coherent_pair_erasure_on_restore_and_historical_subject_reuse_refuse() {
        let (mut runtime, id, calls) = fixture();
        let original = runtime.store.load_record(&id).unwrap();
        let mut erased = original.clone();
        erased.process.pending_outbound_events.clear();
        erased.process.pending_callbacks.clear();
        assert!(matches!(
            InMemoryStore::new().restore_record(erased),
            Err(StoreError::InvalidIntegrationEvidence(_))
        ));
        let mut unmarked = original.clone();
        unmarked.process.pending_outbound_events.clear();
        unmarked.process.pending_callbacks.clear();
        let witness = unmarked
            .provenance_log
            .iter_mut()
            .find(|record| record.record_kind == wos_core::ProvenanceKind::CallbackPending)
            .unwrap();
        witness
            .data
            .as_mut()
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("dispatchMode");
        assert!(matches!(
            InMemoryStore::new().restore_record(unmarked),
            Err(StoreError::InvalidIntegrationEvidence(_))
        ));
        enqueue(&mut runtime, &id, receipt(&original));
        runtime.drain_once(&id).unwrap();
        let mut current = runtime.store.load_record(&id).unwrap();
        let before = serde_json::to_value(&current.process).unwrap();
        let mut binding = profile().bindings.remove("notice").unwrap();
        binding.extensions.insert(
            "subject".into(),
            serde_json::json!(original.process.pending_callbacks.keys().next().unwrap()),
        );
        let observed=wos_core::eval::ObservedAction {
            provenance_record_id: None,
            transition_record_id: None,
            action:serde_json::from_value(serde_json::json!({"action":"invokeService","serviceRef":"notice","callbackPhase":"outbound"})).unwrap(),
            actor_id:None,lifecycle_state:"open".into(),event_data:None,task_expiry_timer:None,
        };
        assert!(
            runtime
                .invoke_integration_binding(
                    &mut current,
                    &kernel("1.0.0"),
                    &observed,
                    "notice",
                    &binding,
                    "2024-03-09T16:00:00Z"
                )
                .is_err()
        );
        assert_eq!(serde_json::to_value(&current.process).unwrap(), before);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
