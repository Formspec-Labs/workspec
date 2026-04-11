// Rust guideline compliant 2026-02-21

//! WOS document parsing and project loading.

use serde_json::Value;
use std::path::Path;

use crate::LintError;

/// Recognized WOS document kinds, detected from `$wos*` markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Kernel,
    WorkflowGovernance,
    DueProcess,
    AssertionLibrary,
    PolicyParameters,
    AiIntegration,
    AgentConfig,
    DriftMonitor,
    Advanced,
    Equity,
    VerificationReport,
    IntegrationProfile,
    SemanticProfile,
    LifecycleDetail,
    CorrespondenceMetadata,
    BusinessCalendar,
    NotificationTemplate,
}

/// A parsed WOS document with its kind and raw JSON value.
#[derive(Debug)]
pub struct WosDocument {
    /// Detected document kind.
    pub kind: DocumentKind,

    /// Raw JSON value.
    pub value: Value,

    /// Source file path, if loaded from disk.
    pub source: Option<String>,
}

/// A collection of WOS documents forming a project.
///
/// Used for Tier 2 cross-document resolution.
///
/// The `skipped` field records any `.json` files that were found but could not
/// be parsed (either invalid JSON or no recognized `$wos*` marker). Callers
/// should inspect this list to detect silent data loss during project loading
/// (Finding #8).
#[derive(Debug, Default)]
pub struct WosProject {
    docs: Vec<WosDocument>,
    /// Files skipped during loading, with their parse error messages.
    pub skipped: Vec<(std::path::PathBuf, String)>,
}

impl WosProject {
    /// All documents in the project.
    pub fn documents(&self) -> &[WosDocument] {
        &self.docs
    }

    /// Find the kernel document, if one exists.
    pub fn kernel(&self) -> Option<&WosDocument> {
        self.docs.iter().find(|d| d.kind == DocumentKind::Kernel)
    }

    /// Find all documents of a given kind.
    pub fn of_kind(&self, kind: DocumentKind) -> impl Iterator<Item = &WosDocument> {
        self.docs.iter().filter(move |d| d.kind == kind)
    }

    /// Add a document to the project.
    pub fn push(&mut self, doc: WosDocument) {
        self.docs.push(doc);
    }
}

// $wos* marker to DocumentKind mapping.
// Key length is intentionally short for fast detection.
const MARKERS: &[(&str, DocumentKind)] = &[
    ("$wosKernel", DocumentKind::Kernel),
    ("$wosWorkflowGovernance", DocumentKind::WorkflowGovernance),
    ("$wosDueProcess", DocumentKind::DueProcess),
    ("$wosAssertionLibrary", DocumentKind::AssertionLibrary),
    ("$wosPolicyParameters", DocumentKind::PolicyParameters),
    ("$wosAIIntegration", DocumentKind::AiIntegration),
    ("$wosAgentConfig", DocumentKind::AgentConfig),
    ("$wosDriftMonitor", DocumentKind::DriftMonitor),
    ("$wosAdvancedGovernance", DocumentKind::Advanced),
    ("$wosEquityConfig", DocumentKind::Equity),
    ("$wosVerificationReport", DocumentKind::VerificationReport),
    ("$wosIntegrationProfile", DocumentKind::IntegrationProfile),
    ("$wosSemanticProfile", DocumentKind::SemanticProfile),
    ("$wosLifecycleDetail", DocumentKind::LifecycleDetail),
    (
        "$wosCorrespondenceMetadata",
        DocumentKind::CorrespondenceMetadata,
    ),
    ("$wosBusinessCalendar", DocumentKind::BusinessCalendar),
    (
        "$wosNotificationTemplate",
        DocumentKind::NotificationTemplate,
    ),
];

/// Parse a JSON string into a `WosDocument`.
///
/// Detects the document kind from `$wos*` markers.
///
/// # Errors
///
/// Returns `LintError::Parse` if the JSON is invalid or no marker is found.
pub fn parse(json: &str) -> Result<WosDocument, LintError> {
    let value: Value = serde_json::from_str(json).map_err(|e| LintError::Parse(e.to_string()))?;

    let obj = value
        .as_object()
        .ok_or_else(|| LintError::Parse("document root must be an object".into()))?;

    let kind = MARKERS
        .iter()
        .find(|(marker, _)| obj.contains_key(*marker))
        .map(|(_, kind)| *kind)
        .ok_or_else(|| LintError::Parse("no recognized $wos* document type marker found".into()))?;

    Ok(WosDocument {
        kind,
        value,
        source: None,
    })
}

/// Load all WOS documents from a directory.
///
/// Recursively scans for `.json` files containing `$wos*` markers. Files that
/// are valid JSON but lack a recognized marker (e.g., plain JSON config files)
/// are silently skipped and recorded in `WosProject::skipped` so callers can
/// inspect the list rather than having data loss go unnoticed (Finding #8).
///
/// # Errors
///
/// Returns `LintError::Io` if the directory cannot be read or a file cannot be
/// opened. Individual parse failures are captured in `WosProject::skipped`
/// rather than propagated as errors.
pub fn load_project(dir: &Path) -> Result<WosProject, LintError> {
    let mut project = WosProject::default();

    for entry in walkdir(dir)? {
        let content = std::fs::read_to_string(&entry)?;
        match parse(&content) {
            Ok(mut doc) => {
                doc.source = Some(entry.to_string_lossy().into_owned());
                project.docs.push(doc);
            }
            Err(err) => {
                // Record the skipped path and the reason so callers can surface it.
                project.skipped.push((entry.clone(), err.to_string()));
            }
        }
    }

    Ok(project)
}

/// Recursively collect `.json` file paths from a directory.
fn walkdir(dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut paths = Vec::new();
    collect_json_files(dir, &mut paths)?;
    Ok(paths)
}

fn collect_json_files(
    dir: &Path,
    paths: &mut Vec<std::path::PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, paths)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            paths.push(path);
        }
    }
    Ok(())
}
