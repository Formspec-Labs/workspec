//! Authors the bounded D1 appeal successor without discarding the original draft.

use serde_json::{Map, Value, json};
use wos_authoring::WosProject;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = std::env::args().skip(1).collect::<Vec<_>>();
    if paths.len() != 2 {
        return Err("usage: d1_appeal_successor ORIGINAL_JSON NEW_EXPORT_JSON".into());
    }
    let original: Value = serde_json::from_slice(&std::fs::read(&paths[0])?)?;
    assert_eq!(original["version"], "0.4.1");
    let mut project =
        WosProject::from_native_document(original.clone()).map_err(|error| error.message)?;
    assert_eq!(project.native_snapshot(), original);
    let pointer = "/lifecycle/states/caseAndDelivery/regions/administrativeReconsideration/states/appealGrantRecorded";
    let mut grant = original
        .pointer(pointer)
        .ok_or("grant state missing")?
        .clone();
    assert_eq!(grant["type"], "atomic");
    assert!(grant.get("transitions").is_none());
    assert_eq!(
        grant["onEntry"],
        json!([
        {"action":"setData","path":"caseFile.caseClosed","value":false},
        {"action":"emitEvent","eventType":"appeal.reopenRequested"}])
    );
    grant["transitions"] = json!([
        {"event":{"kind":"message","name":"appeal.reopenRequested"},"target":"appealClosed"}]);
    project
        .put_state(pointer.into(), grant.clone())
        .map_err(|error| error.message)?;
    project
        .set_native_fields(Map::from_iter([("version".into(), json!("0.4.6"))]))
        .map_err(|error| error.message)?;
    for name in ["appealRequestContract", "appealReviewContract"] {
        let mut contract = original["contracts"][name].clone();
        contract["definitionVersion"] = json!("0.4.1");
        if name == "appealReviewContract" {
            contract["prefillMappingRef"] =
                json!("urn:formspec:demo:d1:passport-appeal-review:mapping:prefill");
        }
        project
            .put_contract(name.into(), contract)
            .map_err(|error| error.message)?;
    }
    let mut expected = original;
    expected["version"] = json!("0.4.6");
    *expected.pointer_mut(pointer).ok_or("grant state missing")? = grant;
    expected["contracts"]["appealRequestContract"]["definitionVersion"] = json!("0.4.1");
    expected["contracts"]["appealReviewContract"]["definitionVersion"] = json!("0.4.1");
    expected["contracts"]["appealReviewContract"]["prefillMappingRef"] =
        json!("urn:formspec:demo:d1:passport-appeal-review:mapping:prefill");
    let exported = project.native_snapshot();
    assert_eq!(
        exported, expected,
        "only the reviewed transition and identity may change"
    );
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&paths[1])?;
    use std::io::Write;
    output.write_all(serde_json::to_string_pretty(&exported)?.as_bytes())?;
    println!(
        "preserved full draft; authored grant transition and successor 0.4.6 and exact appeal reference/prefill bindings through WosProject"
    );
    Ok(())
}
