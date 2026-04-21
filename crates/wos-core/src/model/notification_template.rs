// Rust guideline compliant 2026-04-10

//! Typed model for WOS Notification Template Config sidecars.
//!
//! Deserialized from JSON via serde. Notification template sidecars
//! target a kernel workflow and provide reusable templates for notices
//! generated during governance events: adverse decisions, holds, appeals,
//! SLA warnings, and status updates. Referenced by `notificationTemplateKey`
//! (Governance S12.2) and `noticeTemplateKey` (Governance S3.1).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A WOS Notification Template Config sidecar document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTemplateDocument {
    /// Document type marker. Must be `"1.0"`.
    #[serde(rename = "$wosNotificationTemplate")]
    pub wos_notification_template: String,

    /// Optional JSON Schema URI.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Kernel document this template config targets.
    pub target_workflow: String,

    /// Document version.
    #[serde(default)]
    pub version: Option<String>,

    /// Human-readable title.
    #[serde(default)]
    pub title: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Named notification templates. Keys are the identifiers
    /// referenced by `notificationTemplateKey` and `noticeTemplateKey`.
    pub templates: HashMap<String, NotificationTemplate>,

    /// Extension data. Keys MUST start with `x-`.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// A notification template definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTemplate {
    /// Template category.
    pub category: TemplateCategory,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Subject line. MAY contain `{{variable}}` placeholders.
    #[serde(default)]
    pub subject: Option<String>,

    /// Ordered content sections.
    pub sections: Vec<TemplateSection>,

    /// Variables that MUST be present in the rendering context.
    #[serde(default)]
    pub required_variables: Vec<String>,

    /// Delivery channels.
    #[serde(default)]
    pub delivery_channels: Vec<DeliveryChannel>,

    /// Locale-specific variant reference.
    #[serde(default)]
    pub locale_ref: Option<String>,

    /// Regulatory authority requiring this notification.
    #[serde(default)]
    pub authority: Option<String>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Notification template category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateCategory {
    AdverseDecision,
    HoldNotification,
    AppealAcknowledgment,
    SlaWarning,
    CaseStatusUpdate,
    ResumeNotification,
}

/// Delivery channel for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryChannel {
    Postal,
    Email,
    Portal,
    Sms,
    InApp,
}

/// A section within a notification template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSection {
    /// Section identifier.
    pub id: String,

    /// Section heading.
    #[serde(default)]
    pub title: Option<String>,

    /// Content type.
    pub content_type: SectionContentType,

    /// Content body. MAY contain `{{variable}}` placeholders.
    #[serde(default)]
    pub content: Option<String>,

    /// Whether this section must appear in the rendered notification.
    #[serde(default = "default_true")]
    pub required: bool,

    /// FEL expression controlling section visibility.
    #[serde(default)]
    pub condition: Option<String>,
}

/// Section content type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SectionContentType {
    Text,
    Structured,
    AppealRights,
    ActionRequired,
    ContactInformation,
}

fn default_true() -> bool {
    true
}
