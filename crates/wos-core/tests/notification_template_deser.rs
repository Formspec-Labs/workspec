// Rust guideline compliant 2026-04-10

//! Round-trip deserialization tests for WOS Notification Template Config documents.

use std::fs;
use wos_core::NotificationTemplateDocument;
use wos_core::model::notification_template::{
    TemplateCategory, DeliveryChannel, SectionContentType,
};

fn load_fixture(name: &str) -> NotificationTemplateDocument {
    let path = format!(
        "{}/fixtures/sidecars/{name}",
        env!("CARGO_MANIFEST_DIR").replace("/crates/wos-core", "")
    );
    let json = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {name}: {e}"))
}

#[test]
fn benefits_notification_templates_round_trips() {
    let doc = load_fixture("benefits-notification-templates.json");
    assert_eq!(doc.wos_notification_template, "1.0");
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
    assert!(adverse.required_variables.contains(&"determination".to_string()));
    assert!(adverse.required_variables.contains(&"reasonCodes".to_string()));
    assert!(adverse.required_variables.contains(&"appealDeadline".to_string()));

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
    assert!(hold.required_variables.contains(&"expectedDuration".to_string()));
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
        "$wosNotificationTemplate": "1.0",
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
    assert_eq!(doc.wos_notification_template, "1.0");
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
    let deserialized: NotificationTemplateDocument =
        serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        doc.wos_notification_template,
        deserialized.wos_notification_template
    );
    assert_eq!(doc.templates.len(), deserialized.templates.len());
}
