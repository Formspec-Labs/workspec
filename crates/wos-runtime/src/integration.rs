// Rust guideline compliant 2026-02-21
//! Runtime-compatible reexports of the maintained Core integration model.
#[doc(inline)]
pub use wos_core::model::integration::{
    CallbackPhase, IntegrationBinding, IntegrationBindingKind, IntegrationContractRef,
    IntegrationDispatchMode, IntegrationProfileDocument, TargetWorkflow,
};

#[cfg(test)]
mod tests {
    use super::IntegrationBindingKind;

    /// Verify that `IntegrationBindingKind` round-trips through serde with the
    /// expected kebab-case wire format (e.g., `RequestResponse` → `"request-response"`).
    #[test]
    fn integration_binding_kind_serializes_to_kebab_case() {
        let cases = [
            (IntegrationBindingKind::RequestResponse, "request-response"),
            (IntegrationBindingKind::EventEmit, "event-emit"),
            (IntegrationBindingKind::EventConsume, "event-consume"),
            (IntegrationBindingKind::Callback, "callback"),
            (IntegrationBindingKind::ArazzoSequence, "arazzo-sequence"),
            (IntegrationBindingKind::Tool, "tool"),
            (IntegrationBindingKind::PolicyEngine, "policy-engine"),
        ];

        for (variant, expected_str) in cases {
            let serialized = serde_json::to_value(variant).expect("serialization must succeed");
            assert_eq!(
                serialized.as_str(),
                Some(expected_str),
                "{variant:?} must serialize to \"{expected_str}\""
            );
            let deserialized: IntegrationBindingKind =
                serde_json::from_value(serialized).expect("deserialization must succeed");
            assert_eq!(deserialized, variant, "round-trip must preserve variant");
        }
    }
}
