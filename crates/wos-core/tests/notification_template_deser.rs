// Rust guideline compliant 2026-04-28

//! Round-trip deserialization tests for the notifications content embedded in
//! `$wosDelivery` sidecar documents (was a standalone `$wosNotificationTemplate`
//! sidecar; per ADR 0076 D-3 the marker now lives on the `$wosDelivery`
//! envelope and `NotificationTemplateDocument` represents the embedded
//! `notifications` block).

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use wos_core::NotificationTemplateDocument;
use wos_core::model::notification_template::{
    DeliveryChannel, SectionContentType, TemplateCategory,
};

fn workspace_root() -> PathBuf {
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root is two levels above crates/wos-core")
        .to_path_buf();

    let cwd = std::env::current_dir().ok();
    for candidate in [Some(manifest_root), cwd].into_iter().flatten() {
        for ancestor in candidate.ancestors() {
            if ancestor.join("fixtures").is_dir()
                && ancestor.join("schemas/wos-workflow.schema.json").is_file()
            {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("could not resolve workspace root with fixtures/ and schemas/");
}

fn load_fixture(name: &str) -> NotificationTemplateDocument {
    let path = workspace_root().join("fixtures/sidecars").join(name);
    let json =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    let envelope: Value = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to parse fixture {name} envelope: {e}"));
    assert_eq!(
        envelope.get("$wosDelivery").and_then(Value::as_str),
        Some("1.0"),
        "fixture {name} must carry $wosDelivery envelope per ADR 0076 D-3"
    );
    let mut block = envelope
        .get("notifications")
        .cloned()
        .unwrap_or_else(|| panic!("fixture {name} missing notifications embedded block"));
    if let (Some(map), Some(target)) = (
        block.as_object_mut(),
        envelope.get("targetWorkflow").cloned(),
    ) {
        map.entry("targetWorkflow".to_string()).or_insert(target);
    }
    serde_json::from_value(block)
        .unwrap_or_else(|e| panic!("failed to deserialize notifications from {name}: {e}"))
}

#[test]
fn benefits_notification_templates_round_trips() {
    let doc = load_fixture("benefits-notification-templates.json");
    assert!(doc.target_workflow.contains("benefits-adjudication"));

    // Templates map
    assert_eq!(doc.templates.len(), 4);

    // Adverse decision template
    let adverse = doc
        .templates
        .get("adverseBenefitsDecision")
        .expect("adverse template present");
    assert_eq!(adverse.category, TemplateCategory::AdverseDecision);
    assert!(adverse.subject.is_some());
    assert!(!adverse.sections.is_empty());
    assert!(
        adverse
            .required_variables
            .contains(&"determination".to_string())
    );
    assert!(
        adverse
            .required_variables
            .contains(&"reasonCodes".to_string())
    );
    assert!(
        adverse
            .required_variables
            .contains(&"appealDeadline".to_string())
    );

    // Delivery channels
    assert!(adverse.delivery_channels.contains(&DeliveryChannel::Postal));
    assert!(adverse.delivery_channels.contains(&DeliveryChannel::Portal));

    // Authority
    assert!(adverse.authority.is_some());

    // Section types
    let appeal_rights = adverse
        .sections
        .iter()
        .find(|s| s.content_type == SectionContentType::AppealRights)
        .expect("appeal-rights section present");
    assert_eq!(appeal_rights.id, "appealRights");

    // Conditional section
    let continuation = adverse
        .sections
        .iter()
        .find(|s| s.id == "continuationOfServices")
        .expect("continuation section present");
    assert!(continuation.condition.is_some());
}

#[test]
fn hold_notification_template() {
    let doc = load_fixture("benefits-notification-templates.json");
    let hold = doc
        .templates
        .get("holdPendingDocuments")
        .expect("hold template present");
    assert_eq!(hold.category, TemplateCategory::HoldNotification);
    assert!(hold.required_variables.contains(&"holdReason".to_string()));
    assert!(
        hold.required_variables
            .contains(&"expectedDuration".to_string())
    );
    assert!(hold.delivery_channels.contains(&DeliveryChannel::Email));
}

#[test]
fn sla_warning_template() {
    let doc = load_fixture("benefits-notification-templates.json");
    let sla = doc
        .templates
        .get("slaDeadlineWarning")
        .expect("SLA warning template present");
    assert_eq!(sla.category, TemplateCategory::SlaWarning);
    assert!(sla.required_variables.contains(&"slaDeadline".to_string()));
    assert!(sla.required_variables.contains(&"taskName".to_string()));
    assert!(sla.delivery_channels.contains(&DeliveryChannel::InApp));
}

#[test]
fn notification_template_minimal_document() {
    let json = r#"{
        "targetWorkflow": "https://example.gov/test",
        "templates": {
            "simple": {
                "category": "case-status-update",
                "sections": [
                    {
                        "id": "body",
                        "contentType": "text",
                        "content": "Your case status has been updated."
                    }
                ]
            }
        }
    }"#;
    let doc: NotificationTemplateDocument = serde_json::from_str(json).unwrap();
    assert_eq!(doc.templates.len(), 1);
    let simple = doc.templates.get("simple").unwrap();
    assert_eq!(simple.category, TemplateCategory::CaseStatusUpdate);
    assert_eq!(simple.sections.len(), 1);
    assert_eq!(simple.sections[0].content_type, SectionContentType::Text);
    assert!(simple.required_variables.is_empty());
    assert!(simple.delivery_channels.is_empty());
}

#[test]
fn notification_template_serialization_round_trip() {
    let doc = load_fixture("benefits-notification-templates.json");
    let serialized = serde_json::to_string(&doc).unwrap();
    let deserialized: NotificationTemplateDocument = serde_json::from_str(&serialized).unwrap();
    assert_eq!(doc.templates.len(), deserialized.templates.len());
}
