// Rust guideline compliant 2026-02-21

//! Multi-document project for cross-document resolution.
//!
//! A project holds typed documents from all layers, enabling cross-reference
//! validation (tag matching, actor resolution, targetWorkflow binding).

use crate::model::kernel::KernelDocument;

/// A collection of typed WOS documents.
#[derive(Debug, Default)]
pub struct Project {
    /// The kernel document, if present.
    kernel: Option<KernelDocument>,
    // Governance, AI, and Advanced documents will be added as their
    // typed models are extracted from wos-lint.
}

impl Project {
    /// Set the kernel document.
    pub fn set_kernel(&mut self, kernel: KernelDocument) {
        self.kernel = Some(kernel);
    }

    /// The kernel document, if loaded.
    pub fn kernel(&self) -> Option<&KernelDocument> {
        self.kernel.as_ref()
    }

    /// All actor IDs declared in the kernel.
    pub fn kernel_actor_ids(&self) -> Vec<&str> {
        self.kernel
            .as_ref()
            .map(|k| k.actors.iter().map(|a| a.id.as_str()).collect())
            .unwrap_or_default()
    }

    /// All tags declared on states and transitions in the kernel.
    pub fn kernel_tags(&self) -> std::collections::HashSet<&str> {
        let mut tags = std::collections::HashSet::new();
        if let Some(kernel) = &self.kernel {
            collect_tags(&kernel.lifecycle.states, &mut tags);
        }
        tags
    }

    /// All event names used in kernel transitions.
    pub fn kernel_events(&self) -> std::collections::HashSet<&str> {
        let mut events = std::collections::HashSet::new();
        if let Some(kernel) = &self.kernel {
            collect_events(&kernel.lifecycle.states, &mut events);
        }
        events
    }

    /// All case file field names declared in the kernel.
    pub fn kernel_case_fields(&self) -> std::collections::HashSet<&str> {
        self.kernel
            .as_ref()
            .and_then(|k| k.case_file.as_ref())
            .map(|cf| cf.fields.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

fn collect_tags<'a>(
    states: &'a indexmap::IndexMap<String, crate::model::kernel::State>,
    tags: &mut std::collections::HashSet<&'a str>,
) {
    for state in states.values() {
        for tag in &state.tags {
            tags.insert(tag.as_str());
        }
        for transition in &state.transitions {
            for tag in &transition.tags {
                tags.insert(tag.as_str());
            }
        }
        collect_tags(&state.states, tags);
        for region in state.regions.values() {
            collect_tags(&region.states, tags);
        }
    }
}

fn collect_events<'a>(
    states: &'a indexmap::IndexMap<String, crate::model::kernel::State>,
    events: &mut std::collections::HashSet<&'a str>,
) {
    for state in states.values() {
        for transition in &state.transitions {
            if let Some(ev) = transition.event.as_deref() {
                events.insert(ev);
            }
        }
        collect_events(&state.states, events);
        for region in state.regions.values() {
            collect_events(&region.states, events);
        }
    }
}
