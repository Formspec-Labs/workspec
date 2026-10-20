use super::*;
use stack_common_typeid::testing::fixture_minter;

fn kernel_json() -> serde_json::Value {
    serde_json::json!({
        "$wosWorkflow":"1.0", "url":"urn:test:independent-form-pins", "version":"0.3.0",
        "contracts":{
            "review":{"binding":"formspec","ref":"urn:form:review","definitionVersion":"0.3.2"},
            "correction":{"binding":"formspec","ref":"urn:form:correction","definitionVersion":"0.3.1"},
            "unusedValidation":{"binding":"formspec","ref":"urn:form:validation"}
        },
        "lifecycle":{"initialState":"Review","states":{
            "Review":{"type":"atomic","onEntry":[{"action":"createTask","taskRef":"review",
                "assignTo":"reviewer","contractRef":"review"}],
                "transitions":[{"event":"correct","target":"Correction"}]},
            "Correction":{"type":"atomic","onEntry":[{"action":"createTask","taskRef":"correction",
                "assignTo":"reviewer","contractRef":"correction"}]}
        }}
    })
}

fn request(id: &str) -> CreateProcessRequest {
    CreateProcessRequest {
        process_id: id.into(),
        tenant: None,
        definition_url: "urn:test:independent-form-pins".into(),
        definition_version: "0.3.0".into(),
        initial_case_state: Some(serde_json::json!({})),
    }
}

#[test]
fn heterogeneous_form_pins_survive_reload_and_later_task_creation() {
    let kernel: KernelDocument = serde_json::from_value(kernel_json()).unwrap();
    let mut runtime = runtime_with_kernel(kernel.clone());
    let id = fixture_minter().mint_process_id();
    let created = runtime.create_process(request(&id)).unwrap();
    assert_eq!(created.definition_version, "0.3.0");
    let first = &created.active_tasks[0];
    assert_eq!(first.definition_url.as_deref(), Some("urn:form:review"));
    assert_eq!(first.definition_version.as_deref(), Some("0.3.2"));
    assert_eq!(first.context.as_ref().unwrap().definition_version, "0.3.2");

    let original = runtime.store.load_record(&id).unwrap();
    let aux = crate::runtime_aux_from_json(&crate::runtime_aux_to_json(&original)).unwrap();
    let restored = RuntimeRecord {
        process: serde_json::from_value(serde_json::to_value(&original.process).unwrap()).unwrap(),
        creation: aux.creation,
        provenance_log: original.provenance_log.clone(),
        step_results: aux.step_results,
        artifacts: aux.artifacts,
        replay_entries: aux.replay_entries,
    };
    let mut store = InMemoryStore::new();
    store.restore_record(restored).unwrap();
    // A newer workflow/form cohort exists; the restored process still selects
    // its original immutable workflow and each independently authored form pin.
    let mut newer = kernel_json();
    newer["version"] = "0.4.0".into();
    newer["contracts"]["correction"]["definitionVersion"] = "0.4.0".into();
    let mut reloaded =
        runtime_with_store_kernels(store, vec![kernel, serde_json::from_value(newer).unwrap()]);
    reloaded
        .enqueue_event(
            &id,
            PendingEvent {
                origin: None,
                event: "correct".into(),
                actor_id: None,
                data: None,
                timestamp: String::new(),
                idempotency_token: None,
            },
        )
        .unwrap();
    reloaded.drain_until_idle(&id).unwrap();
    let current = reloaded.store.load_record(&id).unwrap();
    assert_eq!(current.process.definition_version, "0.3.0");
    for (name, version) in [("review", "0.3.2"), ("correction", "0.3.1")] {
        let task = current
            .process
            .active_tasks
            .iter()
            .find(|task| task.task_ref == name)
            .unwrap();
        assert_eq!(task.definition_version.as_deref(), Some(version));
        assert_eq!(task.context.as_ref().unwrap().definition_version, version);
    }
    assert_eq!(current.creation, original.creation);
    assert!(
        current
            .provenance_log
            .iter()
            .filter_map(|record| record.definition_version.as_deref())
            .all(|version| version == "0.3.0")
    );
}

#[test]
fn missing_or_blank_future_form_pin_refuses_before_persistence_or_presentation() {
    for pin in [
        None,
        Some(serde_json::Value::Null),
        Some(serde_json::json!("")),
        Some(serde_json::json!("  ")),
    ] {
        let mut document = kernel_json();
        document["contracts"]["correction"]
            .as_object_mut()
            .unwrap()
            .remove("definitionVersion");
        if let Some(pin) = pin {
            document["contracts"]["correction"]["definitionVersion"] = pin;
        }
        let mut runtime = runtime_with_kernel(serde_json::from_value(document).unwrap());
        let presenter = RecordingPresenter::default();
        let presented = presenter.presented.clone();
        runtime.presenter = Box::new(presenter);
        let id = fixture_minter().mint_process_id();
        let error = runtime.create_process(request(&id)).unwrap_err();
        assert!(
            matches!(error,RuntimeError::MissingMetadata(ref message) if message.contains("correction") && message.contains("definitionVersion"))
        );
        assert!(runtime.store.load_record(&id).is_err());
        assert!(presented.lock().unwrap().is_empty());
    }
}

#[test]
fn future_foreach_body_pins_are_checked_before_process_creation() {
    let action = serde_json::json!({"action":"createTask","taskRef":"correction",
        "assignTo":"reviewer","contractRef":"correction"});
    for body in [
        serde_json::json!({"type":"atomic","onEntry":[action.clone()]}),
        serde_json::json!({"type":"compound","initialState":"Inner",
            "states":{"Inner":{"type":"atomic","onEntry":[action.clone()]}}}),
    ] {
        let mut document = kernel_json();
        document["lifecycle"]["states"]["Correction"] = serde_json::json!({
            "type":"foreach","collection":"[1]","body":body});
        let mut valid = runtime_with_kernel(serde_json::from_value(document.clone()).unwrap());
        assert!(
            valid
                .create_process(request(&fixture_minter().mint_process_id()))
                .is_ok()
        );
        document["contracts"]["correction"]
            .as_object_mut()
            .unwrap()
            .remove("definitionVersion");
        let mut runtime = runtime_with_kernel(serde_json::from_value(document).unwrap());
        let presenter = RecordingPresenter::default();
        let presented = presenter.presented.clone();
        runtime.presenter = Box::new(presenter);
        let id = fixture_minter().mint_process_id();
        assert!(matches!(
            runtime.create_process(request(&id)),
            Err(RuntimeError::MissingMetadata(_))
        ));
        assert!(runtime.store.load_record(&id).is_err());
        assert!(presented.lock().unwrap().is_empty());
    }
}

#[test]
fn migration_refuses_missing_target_form_pin_without_changing_original_evidence() {
    let original: KernelDocument = serde_json::from_value(kernel_json()).unwrap();
    let mut target = kernel_json();
    target["version"] = "0.4.0".into();
    target["contracts"]["correction"]
        .as_object_mut()
        .unwrap()
        .remove("definitionVersion");
    let mut runtime = runtime_with_kernels(vec![original, serde_json::from_value(target).unwrap()]);
    let id = fixture_minter().mint_process_id();
    runtime.create_process(request(&id)).unwrap();
    let before = runtime.store.load_record(&id).unwrap();
    assert!(matches!(
        runtime.migrate(&id, "0.4.0", MigrationMap::default(), None),
        Err(RuntimeError::MigrationRejected(_))
    ));
    let after = runtime.store.load_record(&id).unwrap();
    assert_eq!(
        serde_json::to_value(&after.process).unwrap(),
        serde_json::to_value(&before.process).unwrap()
    );
    assert_eq!(
        crate::runtime_aux_to_json(&after),
        crate::runtime_aux_to_json(&before)
    );
    assert_eq!(
        serde_json::to_value(&after.provenance_log).unwrap(),
        serde_json::to_value(&before.provenance_log).unwrap()
    );
}

#[test]
fn future_compensation_contract_errors_refuse_before_persistence_or_presentation() {
    for missing_contract in [false, true] {
        let mut document = kernel_json();
        document["lifecycle"]["states"]["Correction"]["onEntry"] = serde_json::json!([{
            "action":"log", "message":"future compensated operation",
            "compensatingAction":{"action":"createTask","taskRef":"correction","assignTo":"reviewer","contractRef":"correction"}
        }]);
        if missing_contract {
            document["contracts"]
                .as_object_mut()
                .unwrap()
                .remove("correction");
        } else {
            document["contracts"]["correction"]
                .as_object_mut()
                .unwrap()
                .remove("definitionVersion");
        }
        let mut runtime = runtime_with_kernel(serde_json::from_value(document).unwrap());
        let presenter = RecordingPresenter::default();
        let presented = presenter.presented.clone();
        runtime.presenter = Box::new(presenter);
        let id = stack_common_typeid::testing::fixture_minter().mint_process_id();
        let error = runtime.create_process(request(&id)).unwrap_err();
        assert!(if missing_contract {
            matches!(error, RuntimeError::ContractNotFound(_))
        } else {
            matches!(error, RuntimeError::MissingMetadata(_))
        });
        assert!(runtime.store.load_record(&id).is_err());
        assert!(presented.lock().unwrap().is_empty());
    }
}
