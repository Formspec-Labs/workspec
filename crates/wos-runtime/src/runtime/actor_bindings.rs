// Rust guideline compliant 2026-02-21

//! Validates retained human assignments without granting resource authority.

use super::RuntimeError;
use std::collections::HashSet;
use wos_core::instance::HumanActorBinding;
use wos_core::model::kernel::{ActorKind, KernelDocument};

pub(crate) fn valid_local_actor(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub(crate) fn validate_binding_shape(bindings: Option<&[HumanActorBinding]>) -> Result<(), String> {
    let mut seen = HashSet::new();
    for binding in bindings.into_iter().flatten() {
        let suffix = binding.assigned_actor.strip_prefix("actor:human:");
        if !valid_local_actor(&binding.declared_actor)
            || !seen.insert(&binding.declared_actor)
            || !suffix.is_some_and(|s| {
                !s.is_empty()
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
            })
        {
            return Err("missing, malformed or duplicate human assignment".into());
        }
    }
    Ok(())
}

pub(crate) fn validate_bindings(
    bindings: Option<&[HumanActorBinding]>,
    kernel: &KernelDocument,
) -> Result<(), RuntimeError> {
    validate_binding_shape(bindings).map_err(RuntimeError::ActorBinding)?;
    for binding in bindings.into_iter().flatten() {
        let mut matches = kernel
            .actors
            .iter()
            .filter(|a| a.id == binding.declared_actor);
        if !matches.next().is_some_and(|a| a.kind == ActorKind::Human) || matches.next().is_some() {
            return Err(RuntimeError::ActorBinding(
                "assignment does not name one declared human".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn resolve_assignment(
    bindings: Option<&[HumanActorBinding]>,
    kernel: &KernelDocument,
    declared: Option<&str>,
) -> Result<Option<String>, RuntimeError> {
    let Some(bindings) = bindings else {
        return Ok(declared.map(str::to_string));
    };
    validate_bindings(Some(bindings), kernel)?;
    let Some(declared) = declared else {
        return Ok(None);
    };
    let mut matches = kernel.actors.iter().filter(|a| a.id == declared);
    let actor = matches
        .next()
        .ok_or_else(|| RuntimeError::ActorBinding("task actor is undeclared".into()))?;
    if !valid_local_actor(declared) || matches.next().is_some() {
        return Err(RuntimeError::ActorBinding(
            "task actor declaration is ambiguous or malformed".into(),
        ));
    }
    if actor.kind != ActorKind::Human {
        return Ok(Some(declared.to_string()));
    }
    bindings
        .iter()
        .find(|b| b.declared_actor == declared)
        .map(|b| Some(b.assigned_actor.clone()))
        .ok_or_else(|| RuntimeError::ActorBinding("human task has no resolved assignment".into()))
}

pub(crate) fn validate_binding_definition(
    bindings: Option<&[HumanActorBinding]>,
    kernel: &KernelDocument,
    definition_url: &str,
    definition_version: &str,
) -> Result<(), RuntimeError> {
    if bindings.is_some()
        && (kernel.url.as_deref() != Some(definition_url)
            || kernel.version.as_deref() != Some(definition_version))
    {
        return Err(RuntimeError::ActorBinding(
            "resolved workflow differs from the retained definition".into(),
        ));
    }
    validate_bindings(bindings, kernel)
}
