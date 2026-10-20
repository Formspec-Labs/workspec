// Rust guideline compliant 2026-02-21
//! Exact Response text inside Value-backed storage and buffered command decoding.

use integrity_canonical::ResponseJson;
use serde::{Deserialize, Deserializer, Serializer, de::Error};

pub(crate) fn serialize<S: Serializer>(
    response: &ResponseJson,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(response.as_str())
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<ResponseJson, D::Error> {
    let text = String::deserialize(deserializer)?;
    ResponseJson::parse(text).map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::NativeRuntimeCommand;

    #[test]
    fn task_commands_keep_response_text_through_buffered_value_roundtrip() {
        const TEXT: &str = r#"{"id":"opaque","data":{"decimal":0.1234567890123456789,"integer":9007199254740993}}"#;
        for command in [
            NativeRuntimeCommand::TaskDraft {
                task_id: "task-1".into(),
                response: ResponseJson::parse(TEXT).unwrap(),
                actor_id: "human-1".into(),
                idempotency_token: "draft-1".into(),
            },
            NativeRuntimeCommand::TaskSubmit {
                task_id: "task-1".into(),
                response: ResponseJson::parse(TEXT).unwrap(),
                actor_id: "human-1".into(),
                idempotency_token: "submit-1".into(),
            },
        ] {
            let value = serde_json::to_value(&command).unwrap();
            assert_eq!(value["response"].as_str(), Some(TEXT));
            let restored: NativeRuntimeCommand = serde_json::from_value(value.clone()).unwrap();
            match restored {
                NativeRuntimeCommand::TaskDraft { response, .. }
                | NativeRuntimeCommand::TaskSubmit { response, .. } => {
                    assert_eq!(response.as_str(), TEXT)
                }
                _ => panic!("wrong command"),
            }
            for source in [
                serde_json::json!({"id":"lost"}),
                serde_json::json!("[]"),
                serde_json::json!("{\"id\":1,\"id\":2}"),
            ] {
                let mut invalid = value.clone();
                invalid["response"] = source;
                assert!(serde_json::from_value::<NativeRuntimeCommand>(invalid).is_err());
            }
        }
    }
}
