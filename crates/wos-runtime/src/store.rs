// Rust guideline compliant 2026-02-21

//! Atomic runtime persistence for workflow-process state and task artifacts.

use std::collections::{HashMap, HashSet};

use wos_core::ProvenanceRecord;
use wos_core::instance::WorkflowProcess;

use crate::intake::{
    IntakeAcceptanceDecision, IntakeAcceptanceOutcome, IntakeAcceptanceRequest, IntakeRecordStatus,
    ProcessCreationSnapshot,
};
use crate::runtime::{DismissTaskResult, PersistDraftResult, TaskSubmissionResult};

/// Atomic runtime record for a single workflow process.
#[derive(Debug, Clone)]
pub struct RuntimeRecord {
    /// Canonical WOS workflow process state.
    pub process: WorkflowProcess,
    /// Immutable metadata captured in the original atomic creation write.
    pub creation: Option<ProcessCreationSnapshot>,
    /// Append-only provenance for the process.
    pub provenance_log: Vec<ProvenanceRecord>,
    /// Persisted results from `invokeService` actions.
    pub step_results: Vec<StepResultRecord>,
    /// Draft and accepted task response artifacts.
    pub artifacts: HashMap<String, TaskArtifact>,
    /// Idempotency replay entries for task operations.
    pub replay_entries: HashMap<ReplayKey, ReplayValue>,
}

/// A permitted new custody stamp on an existing provenance record.
///
/// This value describes a validated change, not receipt authenticity or access
/// authority. Storage adapters must persist it atomically and map it on reload
/// without changing the original operational payload or chain digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceCustodyUpdate {
    /// Exact identifier of the retained provenance record.
    pub record_id: String,
    /// Receipt value accepted by the maintained custody stamper.
    pub receipt: wos_events::custody::CustodyAppendReceipt,
}

impl RuntimeRecord {
    /// Create a new record around a freshly-created process.
    pub fn new(instance: WorkflowProcess) -> Self {
        Self {
            process: instance,
            creation: None,
            provenance_log: Vec::new(),
            step_results: Vec::new(),
            artifacts: HashMap::new(),
            replay_entries: HashMap::new(),
        }
    }
}

impl RuntimeRecord {
    /// Checks creation evidence against the exact original staged process.
    ///
    /// Call this at initial persistence, before the process can change.
    /// Original creation evidence is required before initial persistence.
    ///
    /// # Errors
    /// Returns invalid-creation evidence for missing or mismatched original evidence,
    /// invalid replay evidence for missing or conflicting dismissal witnesses,
    /// invalid integration evidence for inconsistent deferred request associations,
    /// or invalid provenance evidence for malformed native presentation acknowledgements.
    pub fn validate_initial_creation(&self) -> Result<(), StoreError> {
        self.creation
            .as_ref()
            .ok_or_else(|| {
                StoreError::InvalidCreationEvidence("original process creation is missing".into())
            })?
            .validate_created_process(&self.process)?;
        crate::runtime::governance::records::validate(self)?;
        self.validate_bound_task_attribution()?;
        self.validate_task_lifetimes()?;
        self.validate_native_presentations()?;
        crate::integration_handlers::callback::validate_pending_record(self)
            .map_err(StoreError::InvalidIntegrationEvidence)?;
        self.validate_record_tenants()?;
        self.validate_dismissal_replay()
    }

    /// Requires every provenance record to carry its process's tenant prefix
    /// (ADR 0068 D-4: the prefix is the scope).
    ///
    /// # Errors
    /// Returns [`StoreError::InvalidCreationEvidence`] when the process's stored
    /// `tenant` differs from its ID prefix, and
    /// [`StoreError::InvalidProvenanceEvidence`] naming the first record whose ID
    /// is not a TypeID of the process's tenant, or when the process ID carries no
    /// tenant.
    pub fn validate_record_tenants(&self) -> Result<(), StoreError> {
        let process_tenant =
            stack_common_typeid::Tenant::of(&self.process.process_id).ok_or_else(|| {
                StoreError::InvalidProvenanceEvidence(format!(
                    "process id `{}` carries no tenant",
                    self.process.process_id
                ))
            })?;
        if process_tenant.as_str() != self.process.tenant {
            return Err(StoreError::InvalidCreationEvidence(
                "process tenant differs from its process id prefix".into(),
            ));
        }
        for record in &self.provenance_log {
            if stack_common_typeid::Tenant::of(&record.id).as_ref() != Some(&process_tenant) {
                return Err(StoreError::InvalidProvenanceEvidence(format!(
                    "provenance record `{}` is not a TypeID of tenant `{process_tenant}`",
                    record.id
                )));
            }
        }
        Ok(())
    }

    /// Checks retained creation identity against the actual loaded process.
    ///
    /// # Errors
    /// Returns invalid-creation evidence for missing or corrupt original evidence,
    /// invalid replay evidence for missing or conflicting dismissal witnesses,
    /// invalid integration evidence for inconsistent deferred request associations,
    /// or invalid provenance evidence for malformed native presentation acknowledgements.
    pub fn validate_creation(&self) -> Result<(), StoreError> {
        self.creation
            .as_ref()
            .ok_or_else(|| {
                StoreError::InvalidCreationEvidence("original process creation is missing".into())
            })?
            .validate_process(&self.process)?;
        crate::runtime::governance::records::validate(self)?;
        self.validate_bound_task_attribution()?;
        self.validate_task_lifetimes()?;
        self.validate_native_presentations()?;
        crate::integration_handlers::callback::validate_pending_record(self)
            .map_err(StoreError::InvalidIntegrationEvidence)?;
        self.validate_dismissal_replay()
    }

    /// Validates original creation evidence before replacing a stored process.
    ///
    /// # Errors
    /// Returns invalid-creation evidence when original metadata changes,
    /// invalid replay evidence when dismissal evidence changes or disappears,
    /// invalid integration evidence when retained deferred requests change or vanish,
    /// or invalid provenance evidence when native acknowledgement witnesses change or vanish.
    pub fn validate_creation_update(&self, previous: &Self) -> Result<(), StoreError> {
        if self.creation != previous.creation {
            return Err(StoreError::InvalidCreationEvidence(
                "original process creation is immutable".into(),
            ));
        }
        self.validate_creation()?;
        self.validate_record_tenants()?;
        crate::runtime::governance::records::validate_update(self, previous)?;
        previous.validate_native_presentations()?;
        let old_presentations: Vec<_> = previous
            .provenance_log
            .iter()
            .filter(|record| native_presentation_details(record).is_some())
            .collect();
        let retained_presentations: Vec<_> = self
            .provenance_log
            .iter()
            .filter(|record| old_presentations.iter().any(|old| old.id == record.id))
            .collect();
        if old_presentations.len() != retained_presentations.len() {
            return Err(invalid_presentation(
                "native presentation evidence was erased or duplicated",
            ));
        }
        for (old, incoming) in old_presentations.into_iter().zip(retained_presentations) {
            validate_retained_provenance(old, incoming)?;
        }
        crate::integration_handlers::callback::validate_pending_update(self, previous)
            .map_err(StoreError::InvalidIntegrationEvidence)?;
        let mut retained_index = HashMap::new();
        for incoming in &self.provenance_log {
            let entry = retained_index
                .entry(incoming.id.as_str())
                .or_insert((incoming, 0usize));
            entry.1 += 1;
        }
        for original in previous.provenance_log.iter().filter(|record| {
            record.record_kind == wos_core::ProvenanceKind::CallbackPending
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("dispatchMode"))
                    .and_then(serde_json::Value::as_str)
                    == Some("deferred")
        }) {
            let (incoming, count) = retained_index.get(original.id.as_str()).ok_or_else(|| {
                StoreError::InvalidIntegrationEvidence(
                    "original deferred callback witness was erased".into(),
                )
            })?;
            if *count != 1 {
                return Err(StoreError::InvalidIntegrationEvidence(
                    "original deferred callback witness is ambiguous".into(),
                ));
            }
            validate_retained_provenance(original, incoming).map_err(|_| {
                StoreError::InvalidIntegrationEvidence(
                    "original deferred callback witness is immutable".into(),
                )
            })?;
        }
        previous.validate_task_lifetimes()?;
        for old_task in &previous.process.active_tasks {
            if let Some(incoming) = self
                .process
                .active_tasks
                .iter()
                .find(|task| task.task_id == old_task.task_id)
            {
                if incoming.expires_at != old_task.expires_at
                    || incoming.expires_with_timer != old_task.expires_with_timer
                {
                    return Err(invalid_attribution("task admission lifetime is immutable"));
                }
            }
        }
        // Lifetime witnesses survive removal/completion of active tasks, including
        // unbound processes and tasks created after the initial snapshot.
        for old in previous
            .provenance_log
            .iter()
            .filter(|p| p.record_kind == wos_core::ProvenanceKind::TaskCreated)
        {
            let incoming = self.provenance_log.iter().find(|p| p.id == old.id);
            if task_lifetime_witness(old)?.is_some()
                || incoming.is_some_and(|p| {
                    p.data
                        .as_ref()
                        .and_then(|d| d.pointer("/details/expiresAt"))
                        .is_some()
                })
            {
                let incoming = incoming
                    .ok_or_else(|| invalid_attribution("task lifetime witness was removed"))?;
                validate_retained_provenance(old, incoming)?;
            }
        }
        let old_lifetime_ids: Vec<_> = previous
            .provenance_log
            .iter()
            .filter(|p| p.record_kind == wos_core::ProvenanceKind::TaskCreated)
            .filter_map(|p| {
                task_lifetime_witness(p)
                    .transpose()
                    .map(|r| r.map(|_| &p.id))
            })
            .collect::<Result<_, _>>()?;
        let retained_lifetime_ids: Vec<_> = self
            .provenance_log
            .iter()
            .filter(|p| old_lifetime_ids.contains(&&p.id))
            .map(|p| &p.id)
            .collect();
        if old_lifetime_ids != retained_lifetime_ids {
            return Err(invalid_attribution("task lifetime witness order changed"));
        }
        if previous.process.actor_bindings.is_some() {
            previous.validate_creation()?;
            let old: Vec<_> = previous
                .provenance_log
                .iter()
                .filter(|p| p.record_kind == wos_core::ProvenanceKind::TaskCreated)
                .collect();
            let new: Vec<_> = self
                .provenance_log
                .iter()
                .filter(|p| p.record_kind == wos_core::ProvenanceKind::TaskCreated)
                .collect();
            if new.len() < old.len() {
                return Err(invalid_attribution("original task attribution was removed"));
            }
            for (old, new) in old.into_iter().zip(new) {
                validate_retained_provenance(old, new)?;
            }
        }
        self.validate_dismissal_replay_update(previous)
    }

    /// Validates retained provenance and returns permitted new custody stamps.
    ///
    /// The caller supplies the actual previous scoped record within its atomic
    /// write. This pure check neither authenticates receipts nor persists them.
    /// It preserves the complete ordered prefix and permits only updates made by
    /// the maintained receipt stamper. Newly appended records are not updates.
    /// Existing creation and dismissal evidence checks remain required.
    ///
    /// # Errors
    /// Returns creation or replay evidence errors from the existing validators,
    /// or [`StoreError::InvalidProvenanceEvidence`] for shortened, ambiguous,
    /// changed or unencodable prior evidence, or conflicting/erased custody hashes.
    pub fn validate_provenance_update(
        &self,
        previous: &Self,
    ) -> Result<Vec<ProvenanceCustodyUpdate>, StoreError> {
        previous.validate_creation()?;
        self.validate_creation_update(previous)?;
        if self.provenance_log.len() < previous.provenance_log.len() {
            return Err(StoreError::InvalidProvenanceEvidence(
                "retained provenance prefix was truncated".into(),
            ));
        }
        let mut identities = HashSet::new();
        for record in &self.provenance_log {
            if record.id.trim().is_empty() || !identities.insert(&record.id) {
                return Err(StoreError::InvalidProvenanceEvidence(
                    "provenance identity is missing or duplicated".into(),
                ));
            }
        }
        let mut changes = Vec::new();
        for (old, incoming) in previous.provenance_log.iter().zip(&self.provenance_log) {
            if let Some(receipt) = validate_retained_provenance(old, incoming)? {
                changes.push(ProvenanceCustodyUpdate {
                    record_id: old.id.clone(),
                    receipt,
                });
            }
        }
        Ok(changes)
    }

    fn validate_native_presentations(&self) -> Result<(), StoreError> {
        let mut ids = HashSet::new();
        let mut intents = HashSet::new();
        for record in &self.provenance_log {
            let Some(details) = native_presentation_details(record) else {
                continue;
            };
            let text = |name| details.get(name).and_then(serde_json::Value::as_str);
            let intent = text("nativePresentationIntentId")
                .ok_or_else(|| invalid_presentation("native presentation intent is missing"))?;
            let digest = text("nativePresentationIntentDigest")
                .ok_or_else(|| invalid_presentation("native presentation digest is missing"))?;
            if record.record_kind != wos_core::ProvenanceKind::TaskPresented
                || !stack_common_typeid::is_valid_type_id(
                    &record.id,
                    Some(stack_common_typeid::PROVENANCE_PREFIX),
                )
                || !ids.insert(&record.id)
                || !intents.insert(intent)
                || !native_presentation_hash(intent)
                || !native_presentation_hash(digest)
                || self
                    .provenance_log
                    .iter()
                    .filter(|other| other.id == record.id)
                    .count()
                    != 1
            {
                return Err(invalid_presentation(
                    "native presentation identity is invalid or ambiguous",
                ));
            }
            let invocation: crate::runtime::NativePresentationInvocationOutcome =
                serde_json::from_value(
                    details
                        .get("nativePresentationInvocation")
                        .cloned()
                        .ok_or_else(|| {
                            invalid_presentation("native presentation invocation is missing")
                        })?,
                )
                .map_err(|_| invalid_presentation("native presentation invocation is invalid"))?;
            let crate::runtime::NativePresentationInvocationOutcome::Invoked {
                attempt_id,
                occurred_at,
            } = invocation
            else {
                return Err(invalid_presentation(
                    "unconfirmed invocation cannot claim TaskPresented",
                ));
            };
            if attempt_id.trim().is_empty()
                || occurred_at != record.timestamp
                || time::OffsetDateTime::parse(
                    &record.timestamp,
                    &time::format_description::well_known::Rfc3339,
                )
                .is_err()
            {
                return Err(invalid_presentation(
                    "native presentation attempt or timestamp is invalid",
                ));
            }
            let witness_id = text("taskCreatedRecordId").ok_or_else(|| {
                invalid_presentation("native presentation creation reference is missing")
            })?;
            let task_id = record
                .data
                .as_ref()
                .and_then(|data| data.get("taskId"))
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| invalid_presentation("native presentation task is missing"))?;
            let witnesses: Vec<_> = self
                .provenance_log
                .iter()
                .filter(|candidate| {
                    candidate.record_kind == wos_core::ProvenanceKind::TaskCreated
                        && candidate
                            .data
                            .as_ref()
                            .and_then(|data| data.get("taskId"))
                            .and_then(serde_json::Value::as_str)
                            == Some(task_id)
                })
                .collect();
            if witnesses.len() != 1
                || witnesses[0].id != witness_id
                || !stack_common_typeid::is_valid_type_id(
                    witness_id,
                    Some(stack_common_typeid::PROVENANCE_PREFIX),
                )
            {
                return Err(invalid_presentation(
                    "native presentation creation witness is invalid or ambiguous",
                ));
            }
            let parse_time = |value: &str| {
                time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
                    .map_err(|_| invalid_presentation("native presentation chronology is invalid"))
            };
            if parse_time(&record.timestamp)? < parse_time(&witnesses[0].timestamp)? {
                return Err(invalid_presentation("presentation precedes task creation"));
            }
        }
        Ok(())
    }

    fn validate_task_lifetimes(&self) -> Result<(), StoreError> {
        let mut witnesses = HashMap::new();
        let mut witness_ids = HashSet::new();
        let mut counts = HashMap::new();
        for record in self
            .provenance_log
            .iter()
            .filter(|p| p.record_kind == wos_core::ProvenanceKind::TaskCreated)
        {
            if let Some(task_id) = record
                .data
                .as_ref()
                .and_then(|d| d.get("taskId"))
                .and_then(serde_json::Value::as_str)
            {
                *counts.entry(task_id).or_insert(0_u64) += 1;
            }
            if let Some((task_id, expires_at, timer_ref)) = task_lifetime_witness(record)? {
                if !stack_common_typeid::is_valid_type_id(
                    &record.id,
                    Some(stack_common_typeid::PROVENANCE_PREFIX),
                ) || chrono::DateTime::parse_from_rfc3339(&record.timestamp).is_err()
                    || !witness_ids.insert(&record.id)
                    || crate::task_process_id(task_id).ok().as_deref()
                        != Some(self.process.process_id.as_str())
                    || witnesses.insert(task_id, (expires_at, timer_ref)).is_some()
                {
                    return Err(invalid_attribution(
                        "task lifetime witness identity is invalid or duplicated",
                    ));
                }
            }
        }
        if witnesses.keys().any(|id| counts.get(id) != Some(&1)) {
            return Err(invalid_attribution(
                "task lifetime has multiple creation witnesses",
            ));
        }
        for task in &self.process.active_tasks {
            validate_task_lifetime(
                task.expires_at.as_deref(),
                task.expires_with_timer.as_deref(),
            )?;
            let retained = witnesses.get(task.task_id.as_str()).copied();
            let current = task
                .expires_at
                .as_deref()
                .zip(task.expires_with_timer.as_deref());
            if retained != current {
                return Err(invalid_attribution(
                    "active task lifetime differs from its original witness",
                ));
            }
        }
        if let Some(creation) = &self.creation {
            for task in &creation.tasks {
                let original = task
                    .expires_at
                    .as_deref()
                    .zip(task.expires_with_timer.as_deref());
                if witnesses.get(task.task_id.as_str()).copied() != original {
                    return Err(invalid_attribution(
                        "original task lifetime differs from its witness",
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_bound_task_attribution(&self) -> Result<(), StoreError> {
        let Some(bindings) = self.process.actor_bindings.as_deref() else {
            return Ok(());
        };
        crate::runtime::actor_bindings::validate_binding_shape(Some(bindings))
            .map_err(|e| invalid_attribution(&e))?;
        let mut witnesses = HashMap::new();
        let mut ids = HashSet::new();
        let mut ordinals = HashSet::new();
        for record in self
            .provenance_log
            .iter()
            .filter(|p| p.record_kind == wos_core::ProvenanceKind::TaskCreated)
        {
            if !stack_common_typeid::is_valid_type_id(
                &record.id,
                Some(stack_common_typeid::PROVENANCE_PREFIX),
            ) || chrono::DateTime::parse_from_rfc3339(&record.timestamp).is_err()
                || !ids.insert(&record.id)
            {
                return Err(invalid_attribution(
                    "task creation provenance identity is missing or duplicated",
                ));
            }
            let data = record
                .data
                .as_ref()
                .ok_or_else(|| invalid_attribution("task creation data is absent"))?;
            let witness: BoundTaskWitness = serde_json::from_value(data.clone())
                .map_err(|_| invalid_attribution("task creation attribution is malformed"))?;
            let details = data
                .get("details")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| invalid_attribution("task creation details are absent"))?;
            if !details.contains_key("binding")
                || !details.contains_key("declaredActor")
                || !details.contains_key("assignedActor")
                || crate::task_process_id(&witness.task_id).ok().as_deref()
                    != Some(self.process.process_id.as_str())
                || witness.details.task_ref.trim().is_empty()
                || witnesses.contains_key(&witness.task_id)
            {
                return Err(invalid_attribution(
                    "task creation attribution is missing, foreign or duplicated",
                ));
            }
            let mut parts = witness.task_id.splitn(4, ':');
            let kind = parts.next();
            let _encoded_process = parts.next();
            let raw_ordinal = parts.next().unwrap_or_default();
            let task_ref = parts.next();
            let ordinal = raw_ordinal
                .parse::<u64>()
                .map_err(|_| invalid_attribution("task creation sequence is malformed"))?;
            if kind != Some("wos-task")
                || ordinal == 0
                || ordinal > self.process.next_task_sequence
                || ordinal.to_string() != raw_ordinal
                || task_ref != Some(witness.details.task_ref.as_str())
                || !ordinals.insert(ordinal)
            {
                return Err(invalid_attribution(
                    "task creation sequence is foreign or duplicated",
                ));
            }
            let expected = match witness.details.declared_actor.as_deref() {
                Some(declared) => {
                    if !crate::runtime::actor_bindings::valid_local_actor(declared) {
                        return Err(invalid_attribution("declared task actor is malformed"));
                    }
                    Some(
                        bindings
                            .iter()
                            .find(|b| b.declared_actor == declared)
                            .map(|b| b.assigned_actor.as_str())
                            .unwrap_or(declared),
                    )
                }
                None => None,
            };
            if witness.details.assigned_actor.as_deref() != expected {
                return Err(invalid_attribution(
                    "original task assignment differs from its retained binding",
                ));
            }
            witnesses.insert(witness.task_id.clone(), witness);
        }
        if u64::try_from(witnesses.len()).ok() != Some(self.process.next_task_sequence) {
            return Err(invalid_attribution(
                "task creation attribution does not cover every created task",
            ));
        }
        let mut active = HashSet::new();
        for task in &self.process.active_tasks {
            let witness = witnesses
                .get(&task.task_id)
                .ok_or_else(|| invalid_attribution("active task has no original attribution"))?;
            if !active.insert(&task.task_id)
                || task.declared_actor != witness.details.declared_actor
                || task.task_ref != witness.details.task_ref
                || task.expires_at != witness.details.expires_at
                || task.expires_with_timer != witness.details.expires_with_timer
            {
                return Err(invalid_attribution(
                    "active task declared attribution differs from its original witness",
                ));
            }
        }
        for task in &self
            .creation
            .as_ref()
            .ok_or_else(|| invalid_attribution("original process creation is absent"))?
            .tasks
        {
            let witness = witnesses
                .get(&task.task_id)
                .ok_or_else(|| invalid_attribution("original task has no retained attribution"))?;
            if task.declared_actor != witness.details.declared_actor
                || task.assigned_actor != witness.details.assigned_actor
                || task.task_ref != witness.details.task_ref
                || task.expires_at != witness.details.expires_at
                || task.expires_with_timer != witness.details.expires_with_timer
            {
                return Err(invalid_attribution(
                    "original creation task attribution conflicts",
                ));
            }
        }
        Ok(())
    }

    /// Validates both directions of retained dismissal operation evidence.
    ///
    /// Every actual dismissal must have one exact replay entry, even after tasks
    /// disappear. Missing entries never become permission to append again.
    ///
    /// # Errors
    /// Returns invalid replay evidence for malformed, missing or conflicting witnesses.
    pub fn validate_dismissal_replay(&self) -> Result<(), StoreError> {
        let mut witnesses = HashMap::new();
        let mut natural_keys = HashSet::new();
        let mut provenance_ids = HashSet::new();
        for provenance in &self.provenance_log {
            if provenance.record_kind != wos_core::ProvenanceKind::TaskDismissed {
                continue;
            }
            let (key, replay) = dismissal_witness(provenance)?;
            let parent = crate::runtime::task_process_id(&key.task_id)
                .map_err(|_| invalid_replay("dismissal task parent is malformed"))?;
            if parent != self.process.process_id
                || !natural_keys.insert((
                    key.task_id.clone(),
                    key.actor_id.clone(),
                    replay.reason.clone(),
                ))
                || !provenance_ids.insert(provenance.id.clone())
                || witnesses.insert(key, replay).is_some()
            {
                return Err(invalid_replay(
                    "dismissal provenance identity is inconsistent or duplicated",
                ));
            }
        }
        for (key, value) in &self.replay_entries {
            match (&key.operation, value) {
                (ReplayOperation::DismissTask, ReplayValue::Dismissal(replay)) => {
                    if witnesses.remove(key).as_ref() != Some(replay) {
                        return Err(invalid_replay(
                            "dismissal replay differs from actual provenance",
                        ));
                    }
                }
                (ReplayOperation::DismissTask, _) | (_, ReplayValue::Dismissal(_)) => {
                    return Err(invalid_replay(
                        "dismissal replay operation and value disagree",
                    ));
                }
                _ => {}
            }
        }
        if !witnesses.is_empty() {
            return Err(invalid_replay(
                "dismissal provenance has no retained replay entry",
            ));
        }
        Ok(())
    }

    /// Preserves retained dismissal evidence against the actual previous record.
    ///
    /// Storage adapters must supply actual previous provenance within their write.
    /// An empty substitute or a read outside that transaction proves no guarantee.
    ///
    /// # Errors
    /// Returns invalid replay evidence for previous corruption, replacement or erasure.
    pub fn validate_dismissal_replay_update(&self, previous: &Self) -> Result<(), StoreError> {
        previous.validate_dismissal_replay()?;
        self.validate_dismissal_replay()?;
        for (key, value) in &previous.replay_entries {
            if let ReplayValue::Dismissal(retained) = value {
                if !matches!(self.replay_entries.get(key), Some(ReplayValue::Dismissal(actual)) if actual == retained)
                {
                    return Err(invalid_replay("retained dismissal replay is immutable"));
                }
            }
        }
        Ok(())
    }
}

fn invalid_replay(message: &str) -> StoreError {
    StoreError::InvalidReplayEvidence(message.to_string())
}

fn validate_dismissal_result(result: &DismissTaskResult) -> Result<(), StoreError> {
    if !stack_common_typeid::is_valid_type_id(
        &result.provenance_id,
        Some(stack_common_typeid::PROVENANCE_PREFIX),
    ) || chrono::DateTime::parse_from_rfc3339(&result.occurred_at).is_err()
    {
        return Err(invalid_replay(
            "dismissal result identity or occurrence time is malformed",
        ));
    }
    Ok(())
}

fn dismissal_witness(
    provenance: &ProvenanceRecord,
) -> Result<(ReplayKey, DismissalReplay), StoreError> {
    let data = provenance
        .data
        .as_ref()
        .ok_or_else(|| invalid_replay("dismissal witness is missing"))?;
    let details = data
        .get("details")
        .ok_or_else(|| invalid_replay("dismissal details are missing"))?;
    if details
        .get("dismissalReplayVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(DISMISSAL_REPLAY_VERSION)
    {
        return Err(invalid_replay(
            "dismissal replay version is missing or unsupported",
        ));
    }
    let field = |value: Option<&serde_json::Value>| {
        value
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| invalid_replay("dismissal witness field is malformed"))
    };
    let task_id = field(data.get("taskId"))?;
    let token = field(details.get("operationToken"))?;
    let actor_id = provenance
        .actor_id
        .clone()
        .filter(|actor| !actor.trim().is_empty())
        .ok_or_else(|| invalid_replay("dismissal actor is missing"))?;
    if token.trim().is_empty() {
        return Err(invalid_replay("dismissal operation token is blank"));
    }
    let replay = DismissalReplay {
        reason: field(details.get("reason"))?,
        result: DismissTaskResult {
            provenance_id: provenance.id.clone(),
            occurred_at: provenance.timestamp.clone(),
        },
    };
    validate_dismissal_result(&replay.result)?;
    Ok((
        ReplayKey {
            operation: ReplayOperation::DismissTask,
            task_id,
            actor_id,
            token,
        },
        replay,
    ))
}

/// Persisted output from an `invokeService` action.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResultRecord {
    /// Exact qualified native input/binding/adapter association, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_association: Option<ServiceInvocationAssociation>,
    /// Service reference declared by the kernel action.
    pub service_ref: String,
    /// Idempotency key used for the invocation, if any.
    pub idempotency_key: Option<String>,
    /// Serialized service output persisted before workflow advance.
    pub output: serde_json::Value,
    /// ISO 8601 persistence timestamp.
    pub recorded_at: String,
}

/// Exact immutable association checked before qualified service-result reuse.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceInvocationAssociation {
    /// Actual constructed native request.
    pub input: serde_json::Value,
    /// Actual selected native integration binding.
    pub binding: serde_json::Value,
    /// Immutable selected adapter and fixture identity.
    pub adapter_identity: String,
}

/// Stored task artifact.
#[derive(Debug, Clone)]
pub struct TaskArtifact {
    /// Stable artifact identifier.
    pub artifact_id: String,
    /// Owning task identifier.
    pub task_id: String,
    /// Artifact kind.
    pub kind: TaskArtifactKind,
    /// Stored response payload.
    pub response: serde_json::Value,
    /// Actor that created the artifact.
    pub actor_id: String,
    /// ISO 8601 persistence timestamp.
    pub recorded_at: String,
}

/// Artifact kind stored by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskArtifactKind {
    /// A draft or amendment artifact.
    Draft,
    /// An accepted completed response.
    Accepted,
}

/// Replay operation category.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReplayOperation {
    /// Draft persistence replay.
    PersistDraft,
    /// Completed submission replay.
    SubmitTaskResponse,
    /// Intake acceptance replay.
    AcceptIntakeHandoff,
    /// Resumable UI dismissal replay.
    DismissTask,
}

/// Idempotency replay key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReplayKey {
    /// Operation category.
    pub operation: ReplayOperation,
    /// Stable task identifier.
    pub task_id: String,
    /// Actor attempting the operation.
    pub actor_id: String,
    /// User-supplied idempotency token.
    pub token: String,
}

/// Idempotent replay value.
#[derive(Debug, Clone)]
pub enum ReplayValue {
    /// Replay of draft persistence.
    Draft(PersistDraftResult),
    /// Replay of task submission.
    Submission(TaskSubmissionResult),
    /// Replay of intake acceptance.
    Intake(IntakeAcceptanceDecision),
    /// Exact request and committed result of a UI dismissal.
    Dismissal(DismissalReplay),
}

/// Retained request and immutable result of a UI dismissal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DismissalReplay {
    /// Exact original reason bytes.
    pub reason: String,
    /// Original actual provenance identity and occurrence time.
    pub result: DismissTaskResult,
}

// Version 1 requires a two-way operation witness; tokenless dismissal records
// cannot be reinterpreted as fresh commands during persisted reload.
pub(crate) const DISMISSAL_REPLAY_VERSION: u64 = 1;

/// Persisted intake-acceptance result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeRecord {
    /// Binding discriminator that interpreted the intake handoff.
    pub binding: String,
    /// Stable idempotency identifier for the intake handoff.
    pub intake_id: String,
    /// Original host-side intake request.
    pub request: IntakeAcceptanceRequest,
    /// Final host-visible outcome.
    pub outcome: IntakeAcceptanceOutcome,
    /// Original creation evidence frozen before intake finalization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation: Option<ProcessCreationSnapshot>,
    /// Provenance emitted for this intake decision.
    pub provenance_log: Vec<ProvenanceRecord>,
    /// Persistence state of this intake record.
    pub status: IntakeRecordStatus,
    /// Timestamp when the intake record was first persisted.
    pub recorded_at: String,
    /// Timestamp of the latest intake-record update.
    pub updated_at: String,
}

impl IntakeRecord {
    /// Checks retained creation evidence against the actual intake request and outcome.
    ///
    /// # Errors
    /// Returns invalid-creation evidence for missing accepted creation, corrupt metadata, or pending policy evidence.
    pub fn validate_creation(&self) -> Result<(), StoreError> {
        if self.status == IntakeRecordStatus::Pending && self.creation.is_some() {
            return Err(StoreError::InvalidCreationEvidence(
                "pending policy cannot supply creation evidence".into(),
            ));
        }
        if self.status != IntakeRecordStatus::Pending
            && matches!(
                &self.outcome,
                IntakeAcceptanceOutcome::Accepted {
                    case_disposition: crate::intake::IntakeCaseDisposition::CreateGovernedCase { .. }
                }
            )
            && self.creation.is_none()
        {
            return Err(StoreError::InvalidCreationEvidence(
                "accepted case creation evidence is missing".into(),
            ));
        }
        if let Some(creation) = &self.creation {
            creation.validate_intake(&self.outcome, &self.request)?;
        }
        Ok(())
    }

    /// Validates creation evidence before replacing a durable intake receipt.
    ///
    /// # Errors
    /// Returns invalid-creation evidence for replacement or conflicting original association.
    pub fn validate_creation_update(&self, previous: &Self) -> Result<(), StoreError> {
        if (previous.status == IntakeRecordStatus::Applied
            && self.status != IntakeRecordStatus::Applied)
            || (previous.status == IntakeRecordStatus::Prepared
                && self.status == IntakeRecordStatus::Pending)
        {
            return Err(StoreError::InvalidCreationEvidence(
                "intake evidence state cannot move backward".into(),
            ));
        }
        if previous.status != IntakeRecordStatus::Pending && self.creation != previous.creation {
            return Err(StoreError::InvalidCreationEvidence(
                "prepared intake creation is immutable".into(),
            ));
        }
        self.validate_creation()
    }
}

/// Errors from runtime persistence.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StoreError {
    /// Record not found.
    #[error("runtime record not found: {0}")]
    NotFound(String),

    /// Record already exists.
    #[error("runtime record already exists: {0}")]
    AlreadyExists(String),

    /// Original creation evidence is corrupt or an update attempted to replace it.
    #[error("invalid creation evidence: {0}")]
    InvalidCreationEvidence(String),

    /// Retained dismissal request, result or provenance evidence is inconsistent.
    #[error("invalid replay evidence: {0}")]
    InvalidReplayEvidence(String),
    /// A retained provenance prefix or custody update is inconsistent.
    #[error("invalid provenance evidence: {0}")]
    InvalidProvenanceEvidence(String),

    /// A retained deferred request/receipt association is corrupt or changed.
    #[error("invalid integration evidence: {0}")]
    InvalidIntegrationEvidence(String),

    /// Store-specific failure.
    #[error("runtime store failure: {0}")]
    Failed(String),
}

/// Atomic runtime record store.
pub trait RuntimeStore {
    /// Create a brand-new workflow-process record.
    /// Implementations must call [`RuntimeRecord::validate_initial_creation`] inside the initial atomic write.
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError>;

    /// Load the current record for a workflow process.
    fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError>;

    /// Return the `process_id`s of every record bound to `case_ledger_id`.
    ///
    /// Empty vector when no records match.
    fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String>;

    /// Return every provenance record across every workflow process bound to
    /// `case_ledger_id`, merged in `(timestamp, encounter_order)` order.
    ///
    /// This is the N:1 traversal primitive — one case file aggregating events
    /// from many workflow processes (see case-boundary report §4.3). Records
    /// `encounter_order` is the case-index process traversal order plus each
    /// process log's insertion order. Records with an empty `timestamp`
    /// (unit-test fixtures that never reached the runtime stamper) sort to the
    /// front in encounter order — exporters should treat empty timestamps as
    /// "unknown" per
    /// [`ProvenanceRecord`] docs.
    ///
    /// Empty vector when no processes are bound to the case ledger or when
    /// every bound process has an empty provenance log. Default implementation
    /// returns empty; in-memory and storage-backed adapters override.
    fn provenance_for_case(&self, _case_ledger_id: &str) -> Vec<ProvenanceRecord> {
        Vec::new()
    }

    /// Append a provenance record to the workflow process identified by
    /// `process_id` after validating that the process is bound to
    /// `case_ledger_id`.
    ///
    /// Records are stored on the per-process `provenance_log`; this method
    /// exists as the case-scoped writer counterpart to [`Self::provenance_for_case`]
    /// so callers traversing case-keyed routes (ADR-0093 §2.8) need not load
    /// the record first. Returns [`StoreError::NotFound`] when the process is
    /// missing or bound to a different case ledger. Storage adapters MUST
    /// implement this explicitly so their atomicity and index-consistency
    /// behavior is visible at the adapter boundary.
    ///
    /// # Errors
    /// Returns an error when the record cannot be loaded, the case-ledger
    /// binding mismatches, or the save fails.
    fn append_provenance_for_case(
        &mut self,
        case_ledger_id: &str,
        process_id: &str,
        record: ProvenanceRecord,
    ) -> Result<(), StoreError>;

    /// Atomically replace a workflow-process record.
    /// Implementations must preserve the original creation snapshot exactly, without replacement or erasure.
    /// Use [`RuntimeRecord::validate_creation_update`] before replacing persisted state.
    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError>;

    /// Create a brand-new intake record.
    fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError>;

    /// Load a persisted intake record.
    fn load_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> Result<IntakeRecord, StoreError>;

    /// Atomically replace an intake record.
    /// Use [`IntakeRecord::validate_creation_update`] to preserve Prepared/Applied creation evidence.
    fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError>;
}

fn append_provenance_via_process_round_trip<S: RuntimeStore + ?Sized>(
    store: &mut S,
    case_ledger_id: &str,
    process_id: &str,
    record: ProvenanceRecord,
) -> Result<(), StoreError> {
    let mut runtime_record = store.load_record(process_id)?;
    if runtime_record.process.case_ledger_id != case_ledger_id {
        return Err(StoreError::NotFound(format!(
            "process `{process_id}` is not bound to case ledger `{case_ledger_id}`"
        )));
    }
    runtime_record.provenance_log.push(record);
    runtime_record.process.provenance_position = runtime_record.provenance_log.len() as u64;
    store.save_record(runtime_record)
}

/// In-memory runtime record store.
///
/// Maintains a secondary `case_ledger_id` → `Vec<process_id>` index so
/// case-scoped traversals (N:1 case-to-processes) run in O(processes-per-case)
/// rather than O(all-processes-in-store). Insertion order is preserved within
/// each case for callers that opt out of timestamp sorting.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    records: HashMap<String, RuntimeRecord>,
    case_index: HashMap<String, Vec<String>>,
    intake_records: HashMap<(String, String), IntakeRecord>,
}

impl InMemoryStore {
    /// Create an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restore a persisted current record into this store.
    ///
    /// Use this only for records loaded from trusted persistence after validating
    /// the actual storage key and scope. This method grants no access authority
    /// and must not replace initial creation or intake admission.
    ///
    /// Retained creation evidence is checked against current process identity;
    /// current task progress and definition-version migration are permitted.
    /// Original creation evidence is required. Existing records are never replaced.
    ///
    /// # Errors
    /// Returns [`StoreError::InvalidCreationEvidence`] for invalid retained evidence
    /// or an inconsistent process association, [`StoreError::InvalidReplayEvidence`]
    /// for malformed or inconsistent dismissal evidence, and [`StoreError::AlreadyExists`]
    /// when the process is already present. Refusal leaves records and indexes unchanged.
    pub fn restore_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        record.validate_creation()?;
        record.validate_record_tenants()?;
        self.insert_record(record)
    }

    fn insert_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let process_id = record.process.process_id.clone();
        if self.records.contains_key(&process_id) {
            return Err(StoreError::AlreadyExists(process_id));
        }

        let case_ledger_id = record.process.case_ledger_id.clone();
        self.records.insert(process_id.clone(), record);
        self.case_index
            .entry(case_ledger_id)
            .or_default()
            .push(process_id);
        Ok(())
    }
}

impl RuntimeStore for InMemoryStore {
    fn create_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        record.validate_initial_creation()?;
        self.insert_record(record)
    }

    fn load_record(&self, process_id: &str) -> Result<RuntimeRecord, StoreError> {
        self.records
            .get(process_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(process_id.to_string()))
    }

    fn processes_for_case(&self, case_ledger_id: &str) -> Vec<String> {
        self.case_index
            .get(case_ledger_id)
            .cloned()
            .unwrap_or_default()
    }

    fn provenance_for_case(&self, case_ledger_id: &str) -> Vec<ProvenanceRecord> {
        let Some(process_ids) = self.case_index.get(case_ledger_id) else {
            return Vec::new();
        };
        // (timestamp, encounter_order, record) — encounter order preserves
        // intra-process insertion order and keeps the sort stable for the
        // empty-timestamp fixture case.
        let mut merged: Vec<(String, usize, ProvenanceRecord)> = Vec::new();
        let mut counter: usize = 0;
        for process_id in process_ids {
            let Some(record) = self.records.get(process_id) else {
                continue;
            };
            for provenance in &record.provenance_log {
                merged.push((provenance.timestamp.clone(), counter, provenance.clone()));
                counter += 1;
            }
        }
        merged.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        merged.into_iter().map(|(_, _, record)| record).collect()
    }

    fn append_provenance_for_case(
        &mut self,
        case_ledger_id: &str,
        process_id: &str,
        record: ProvenanceRecord,
    ) -> Result<(), StoreError> {
        append_provenance_via_process_round_trip(self, case_ledger_id, process_id, record)
    }

    fn save_record(&mut self, record: RuntimeRecord) -> Result<(), StoreError> {
        let process_id = record.process.process_id.clone();
        let Some(existing) = self.records.get(&process_id) else {
            return Err(StoreError::NotFound(process_id));
        };
        if existing.process.case_ledger_id != record.process.case_ledger_id {
            return Err(StoreError::Failed(format!(
                "case ledger binding for process `{process_id}` is immutable"
            )));
        }

        record.validate_creation_update(existing)?;
        self.records.insert(process_id, record);
        Ok(())
    }

    fn create_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        record.validate_creation()?;
        let key = (record.binding.clone(), record.intake_id.clone());
        if self.intake_records.contains_key(&key) {
            return Err(StoreError::AlreadyExists(format!(
                "intake:{}:{}",
                key.0, key.1
            )));
        }
        self.intake_records.insert(key, record);
        Ok(())
    }

    fn load_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> Result<IntakeRecord, StoreError> {
        self.intake_records
            .get(&(binding.to_string(), intake_id.to_string()))
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("intake:{binding}:{intake_id}")))
    }

    fn save_intake_record(&mut self, record: IntakeRecord) -> Result<(), StoreError> {
        let key = (record.binding.clone(), record.intake_id.clone());
        let previous = self
            .intake_records
            .get(&key)
            .ok_or_else(|| StoreError::NotFound(format!("intake:{}:{}", key.0, key.1)))?;
        record.validate_creation_update(previous)?;
        self.intake_records.insert(key, record);
        Ok(())
    }
}

/// Step results, artifacts, and replay map split out of [`RuntimeRecord`] for auxiliary storage.
#[derive(Debug, Clone, Default)]
pub struct RuntimeAuxFields {
    /// Original creation evidence; stores must preserve it on every reload/save.
    pub creation: Option<ProcessCreationSnapshot>,
    /// Persisted `invokeService` outputs.
    pub step_results: Vec<StepResultRecord>,
    /// Task response artifacts.
    pub artifacts: HashMap<String, TaskArtifact>,
    /// Idempotent replay entries.
    pub replay_entries: HashMap<ReplayKey, ReplayValue>,
}

/// Serializes non-instance fields of [`RuntimeRecord`] for auxiliary JSON storage (SBA `runtime_aux_json` shape).
pub fn runtime_aux_to_json(record: &RuntimeRecord) -> serde_json::Value {
    let step_results: Vec<_> = record
        .step_results
        .iter()
        .map(|s| {
            serde_json::json!({
                "serviceRef": s.service_ref,
                "idempotencyKey": s.idempotency_key,
                "output": s.output,
                "invocationAssociation": s.invocation_association,
                "recordedAt": s.recorded_at,
            })
        })
        .collect();
    let artifacts: serde_json::Map<String, serde_json::Value> = record
        .artifacts
        .iter()
        .map(|(k, a)| {
            let kind = match a.kind {
                TaskArtifactKind::Draft => "draft",
                TaskArtifactKind::Accepted => "accepted",
            };
            (
                k.clone(),
                serde_json::json!({
                    "artifactId": a.artifact_id,
                    "taskId": a.task_id,
                    "kind": kind,
                    "response": a.response,
                    "actorId": a.actor_id,
                    "recordedAt": a.recorded_at,
                }),
            )
        })
        .collect();
    let replay_entries: Vec<serde_json::Value> = record
        .replay_entries
        .iter()
        .map(|(k, v)| {
            let op = match k.operation {
                ReplayOperation::PersistDraft => "persistDraft",
                ReplayOperation::SubmitTaskResponse => "submitTaskResponse",
                ReplayOperation::AcceptIntakeHandoff => "acceptIntakeHandoff",
                ReplayOperation::DismissTask => "dismissTask",
            };
            let value_json = match v {
                ReplayValue::Draft(d) => serde_json::json!({
                    "kind": "draft",
                    "artifactId": d.artifact_id,
                }),
                ReplayValue::Submission(s) => match s {
                    TaskSubmissionResult::Completed {
                        artifact_id,
                        case_mutated,
                        emitted_event,
                    } => serde_json::json!({
                        "kind": "completed",
                        "artifactId": artifact_id,
                        "caseMutated": case_mutated,
                        "emittedEvent": emitted_event,
                    }),
                    TaskSubmissionResult::Failed {
                        code,
                        emitted_event,
                    } => serde_json::json!({
                        "kind": "failed",
                        "code": code,
                        "emittedEvent": emitted_event,
                    }),
                    TaskSubmissionResult::Rejected { code } => {
                        serde_json::json!({ "kind": "rejected", "code": code })
                    }
                },
                ReplayValue::Dismissal(replay) => serde_json::json!({
                    "kind": "dismissal", "reason": replay.reason, "result": replay.result,
                }),
                ReplayValue::Intake(decision) => {
                    let decision_json =
                        serde_json::to_value(decision).unwrap_or(serde_json::json!({}));
                    serde_json::json!({
                        "kind": "intake",
                        "decision": decision_json,
                    })
                }
            };
            serde_json::json!({
                "key": {
                    "operation": op,
                    "taskId": k.task_id,
                    "actorId": k.actor_id,
                    "token": k.token,
                },
                "value": value_json,
            })
        })
        .collect();
    serde_json::json!({
        "creation": record.creation,
        "stepResults": step_results,
        "artifacts": artifacts,
        "replayEntries": replay_entries,
    })
}

/// Deserializes auxiliary JSON produced by [`runtime_aux_to_json`].
///
/// Original process creation evidence is required; missing, null, or malformed evidence is refused.
///
/// # Errors
/// Returns invalid-creation evidence for missing, null, malformed, or inconsistent original metadata,
/// or invalid replay evidence for malformed or duplicate dismissal entries.
pub fn runtime_aux_from_json(v: &serde_json::Value) -> Result<RuntimeAuxFields, StoreError> {
    let creation: ProcessCreationSnapshot =
        serde_json::from_value(v.get("creation").cloned().ok_or_else(|| {
            StoreError::InvalidCreationEvidence("original process creation is missing".into())
        })?)
        .map_err(|error| {
            StoreError::InvalidCreationEvidence(format!("creation snapshot decode: {error}"))
        })?;
    creation.validate()?;
    let step_results = v
        .get("stepResults")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(StepResultRecord {
                        invocation_association: s
                            .get("invocationAssociation")
                            .filter(|value| !value.is_null())
                            .cloned()
                            .map(serde_json::from_value)
                            .transpose()
                            .ok()?,
                        service_ref: s.get("serviceRef")?.as_str()?.to_string(),
                        idempotency_key: s
                            .get("idempotencyKey")
                            .and_then(|x| x.as_str())
                            .map(String::from),
                        output: s.get("output").cloned().unwrap_or(serde_json::json!({})),
                        recorded_at: s
                            .get("recordedAt")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let artifacts = v
        .get("artifacts")
        .and_then(|x| x.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, a)| {
                    let kind = match a.get("kind").and_then(|x| x.as_str())? {
                        "draft" => TaskArtifactKind::Draft,
                        "accepted" => TaskArtifactKind::Accepted,
                        _ => return None,
                    };
                    Some((
                        k.clone(),
                        TaskArtifact {
                            artifact_id: a.get("artifactId")?.as_str()?.to_string(),
                            task_id: a.get("taskId")?.as_str()?.to_string(),
                            kind,
                            response: a.get("response").cloned().unwrap_or(serde_json::json!({})),
                            actor_id: a
                                .get("actorId")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            recorded_at: a
                                .get("recordedAt")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    // Validate known dismissal entries before the existing permissive codecs
    // or HashMap collection can discard malformed or duplicate evidence.
    let dismissals = decode_dismissal_replays(v)?;
    let mut replay_entries: HashMap<ReplayKey, ReplayValue> = v
        .get("replayEntries")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let k = entry.get("key")?;
                    let val = entry.get("value")?;
                    let op = match k.get("operation").and_then(|x| x.as_str())? {
                        "persistDraft" => ReplayOperation::PersistDraft,
                        "submitTaskResponse" => ReplayOperation::SubmitTaskResponse,
                        "acceptIntakeHandoff" => ReplayOperation::AcceptIntakeHandoff,
                        _ => return None,
                    };
                    let key = ReplayKey {
                        operation: op,
                        task_id: k.get("taskId")?.as_str()?.to_string(),
                        actor_id: k.get("actorId")?.as_str()?.to_string(),
                        token: k.get("token")?.as_str()?.to_string(),
                    };
                    let value = match val.get("kind").and_then(|x| x.as_str())? {
                        "draft" => ReplayValue::Draft(PersistDraftResult {
                            artifact_id: val.get("artifactId")?.as_str()?.to_string(),
                        }),
                        "completed" => ReplayValue::Submission(TaskSubmissionResult::Completed {
                            artifact_id: val.get("artifactId")?.as_str()?.to_string(),
                            case_mutated: val
                                .get("caseMutated")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(false),
                            emitted_event: val
                                .get("emittedEvent")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        }),
                        "failed" => ReplayValue::Submission(TaskSubmissionResult::Failed {
                            code: val.get("code")?.as_str()?.to_string(),
                            emitted_event: val
                                .get("emittedEvent")
                                .and_then(|x| x.as_str())
                                .map(String::from),
                        }),
                        "rejected" => ReplayValue::Submission(TaskSubmissionResult::Rejected {
                            code: val.get("code")?.as_str()?.to_string(),
                        }),
                        "intake" => {
                            let decision_val = val.get("decision")?.clone();
                            let decision: IntakeAcceptanceDecision =
                                serde_json::from_value(decision_val).ok()?;
                            ReplayValue::Intake(decision)
                        }
                        _ => return None,
                    };
                    Some((key, value))
                })
                .collect()
        })
        .unwrap_or_default();
    replay_entries.extend(dismissals);
    Ok(RuntimeAuxFields {
        creation: Some(creation),
        step_results,
        artifacts,
        replay_entries,
    })
}

fn decode_dismissal_replays(
    v: &serde_json::Value,
) -> Result<HashMap<ReplayKey, ReplayValue>, StoreError> {
    let mut entries = HashMap::new();
    if let Some(raw) = v.get("replayEntries").and_then(serde_json::Value::as_array) {
        for entry in raw {
            let key = entry.get("key");
            let value = entry.get("value");
            let known = key
                .and_then(|k| k.get("operation"))
                .and_then(serde_json::Value::as_str)
                == Some("dismissTask")
                || value
                    .and_then(|v| v.get("kind"))
                    .and_then(serde_json::Value::as_str)
                    == Some("dismissal");
            if !known {
                continue;
            }
            let key = key
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| invalid_replay("dismissal key is malformed"))?;
            let value = value
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| invalid_replay("dismissal value is malformed"))?;
            if entry.as_object().map(|o| o.len()) != Some(2)
                || key.len() != 4
                || value.len() != 3
                || key.get("operation").and_then(serde_json::Value::as_str) != Some("dismissTask")
                || value.get("kind").and_then(serde_json::Value::as_str) != Some("dismissal")
            {
                return Err(invalid_replay(
                    "dismissal replay shape or operation is inconsistent",
                ));
            }
            let field = |name: &str| {
                key.get(name)
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| invalid_replay("dismissal key field is malformed"))
            };
            let key = ReplayKey {
                operation: ReplayOperation::DismissTask,
                task_id: field("taskId")?,
                actor_id: field("actorId")?,
                token: field("token")?,
            };
            let replay: DismissalReplay = serde_json::from_value(
                serde_json::json!({"reason": value.get("reason"), "result": value.get("result")}),
            )
            .map_err(|_| invalid_replay("dismissal request or result is malformed"))?;
            validate_dismissal_result(&replay.result)?;
            if entries
                .insert(key, ReplayValue::Dismissal(replay))
                .is_some()
            {
                return Err(invalid_replay("duplicate dismissal replay key"));
            }
        }
    }
    Ok(entries)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundTaskWitness {
    task_id: String,
    details: BoundTaskDetails,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundTaskDetails {
    task_ref: String,
    #[serde(rename = "binding")]
    _binding: Option<String>,
    declared_actor: Option<String>,
    assigned_actor: Option<String>,
    expires_at: Option<String>,
    expires_with_timer: Option<String>,
    #[serde(rename = "calendarVersion")]
    _calendar_version: Option<String>,
    #[serde(rename = "calendarVersionConvergenceError")]
    _calendar_version_convergence_error: Option<bool>,
}

/// Validate a paired task lifetime without granting command authority.
pub(crate) fn validate_task_lifetime(
    expires_at: Option<&str>,
    timer_ref: Option<&str>,
) -> Result<Option<u64>, StoreError> {
    match (expires_at, timer_ref) {
        (None, None) => Ok(None),
        (Some(timestamp), Some(timer_ref)) if !timer_ref.trim().is_empty() => {
            let parsed = time::OffsetDateTime::parse(
                timestamp,
                &time::format_description::well_known::Rfc3339,
            )
            .map_err(|_| invalid_attribution("task expiry timestamp is malformed"))?;
            let nanos = parsed.unix_timestamp_nanos();
            if nanos < 0 {
                return Err(invalid_attribution(
                    "task expiry timestamp is out of supported range",
                ));
            }
            Ok(Some(u64::try_from(nanos / 1_000_000).map_err(|_| {
                invalid_attribution("task expiry timestamp is out of range")
            })?))
        }
        _ => Err(invalid_attribution(
            "task expiry requires paired timestamp and nonblank timer reference",
        )),
    }
}

/// Extract an explicitly configured lifetime from original task creation evidence.
fn task_lifetime_witness(
    record: &ProvenanceRecord,
) -> Result<Option<(&str, &str, &str)>, StoreError> {
    let Some(details) = record
        .data
        .as_ref()
        .and_then(|data| data.get("details"))
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(None);
    };
    if !details.contains_key("expiresAt") && !details.contains_key("expiresWithTimer") {
        return Ok(None);
    }
    let task_id = record
        .data
        .as_ref()
        .and_then(|data| data.get("taskId"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_attribution("task lifetime witness is missing taskId"))?;
    let expires_at = details.get("expiresAt").and_then(serde_json::Value::as_str);
    let timer_ref = details
        .get("expiresWithTimer")
        .and_then(serde_json::Value::as_str);
    validate_task_lifetime(expires_at, timer_ref)?;
    match (expires_at, timer_ref) {
        (Some(expires_at), Some(timer_ref)) => Ok(Some((task_id, expires_at, timer_ref))),
        _ => Err(invalid_attribution(
            "task lifetime witness fields are malformed",
        )),
    }
}

fn invalid_attribution(message: &str) -> StoreError {
    StoreError::InvalidCreationEvidence(message.into())
}

// Only the new native acknowledgement marker opts into these checks. Legacy
// TaskPresented rows retain their existing format and are never relabeled.
fn native_presentation_details(record: &ProvenanceRecord) -> Option<&serde_json::Value> {
    let details = record.data.as_ref()?.get("details")?;
    [
        "nativePresentationIntentId",
        "nativePresentationIntentDigest",
        "nativePresentationInvocation",
    ]
    .iter()
    .any(|key| details.get(*key).is_some())
    .then_some(details)
}
fn native_presentation_hash(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
fn invalid_presentation(message: &str) -> StoreError {
    StoreError::InvalidProvenanceEvidence(message.into())
}

fn validate_retained_provenance(
    old: &ProvenanceRecord,
    incoming: &ProvenanceRecord,
) -> Result<Option<wos_events::custody::CustodyAppendReceipt>, StoreError> {
    let mut expected = old.clone();
    let mut new_receipt = None;
    if let Some(hash) = &incoming.canonical_event_hash {
        let receipt = wos_events::custody::CustodyAppendReceipt::new(hash.clone());
        crate::runtime::stamp_custody_receipt(&mut expected, &receipt)
            .map_err(|error| StoreError::InvalidProvenanceEvidence(error.to_string()))?;
        if old.canonical_event_hash.is_none() {
            new_receipt = Some(receipt);
        }
    } else if old.canonical_event_hash.is_some() {
        return Err(StoreError::InvalidProvenanceEvidence(
            "retained custody receipt was erased".into(),
        ));
    }
    let expected_json = serde_json::to_value(&expected)
        .map_err(|error| StoreError::InvalidProvenanceEvidence(error.to_string()))?;
    let incoming_json = serde_json::to_value(incoming)
        .map_err(|error| StoreError::InvalidProvenanceEvidence(error.to_string()))?;
    if expected_json != incoming_json {
        return Err(StoreError::InvalidProvenanceEvidence(
            "retained provenance differs beyond its permitted custody stamp".into(),
        ));
    }
    Ok(new_receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stack_common_typeid::testing::fixture_minter;
    use wos_core::ProvenanceRecord;
    use wos_core::instance::InstanceStatus;

    fn creation_record() -> RuntimeRecord {
        let process = fixture_minter().mint_process_id();
        let case = fixture_minter().mint_case_ledger_id();
        let mut record = record_with(&process, &case);
        let kernel = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": record.process.definition_url, "version": "1.0.0",
            "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic" } } }
        }))
        .unwrap();
        record.creation = Some(
            crate::intake::ProcessCreationSnapshot::capture(&record.process, &kernel).unwrap(),
        );
        record
    }

    #[test]
    fn creation_snapshot_aux_refuses_missing_null_and_corrupt_evidence() {
        for aux in [serde_json::json!({}), serde_json::json!({"creation": null})] {
            assert!(matches!(
                runtime_aux_from_json(&aux),
                Err(StoreError::InvalidCreationEvidence(_))
            ));
        }
        assert!(matches!(
            runtime_aux_from_json(&serde_json::json!({ "creation": { "tasks": [] } })),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let record = creation_record();
        let mut aux = runtime_aux_to_json(&record);
        assert_eq!(
            runtime_aux_from_json(&aux).unwrap().creation,
            record.creation
        );
        aux["creation"]["processId"] = serde_json::json!(record.process.case_ledger_id);
        assert!(matches!(
            runtime_aux_from_json(&aux),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
    }

    #[test]
    fn creation_snapshot_process_store_refuses_replacement_and_erasure() {
        let mut store = InMemoryStore::new();
        let original = creation_record();
        let id = original.process.process_id.clone();
        store.create_record(original.clone()).unwrap();
        let mut changed = original.clone();
        changed.creation.as_mut().unwrap().definition_version = "2.0.0".into();
        assert!(matches!(
            store.save_record(changed),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut erased = original.clone();
        erased.creation = None;
        assert!(matches!(
            store.save_record(erased),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        assert_eq!(store.load_record(&id).unwrap().creation, original.creation);
    }

    #[test]
    fn creation_snapshot_initial_store_refuses_wrong_original_version_and_tasks() {
        let mut store = InMemoryStore::new();
        let mut wrong_version = creation_record();
        wrong_version.creation.as_mut().unwrap().definition_version = "2.0.0".into();
        assert!(matches!(
            store.create_record(wrong_version),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut wrong_set = creation_record();
        wrong_set
            .process
            .active_tasks
            .push(wos_core::instance::ActiveTask {
                declared_actor: None,
                task_id: "not-the-captured-task".into(),
                task_ref: "review".into(),
                status: wos_core::instance::ActiveTaskStatus::Assigned,
                assigned_actor: Some("reviewer".into()),
                contract_ref: None,
                binding: None,
                definition_url: None,
                definition_version: None,
                prefill_mapping_ref: None,
                response_mapping_ref: None,
                deadline: None,
                expires_at: None,
                expires_with_timer: None,
                impact_level: None,
                context: None,
                last_validation_outcome: None,
                created_at: wrong_set.process.created_at.clone(),
                updated_at: wrong_set.process.created_at.clone(),
                extensions: Default::default(),
            });
        assert!(matches!(
            store.create_record(wrong_set),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
    }

    #[test]
    fn creation_snapshot_update_preserves_original_pin_during_version_migration() {
        let mut store = InMemoryStore::new();
        let original = creation_record();
        store.create_record(original.clone()).unwrap();
        let mut migrated = original.clone();
        migrated.process.definition_version = "2.0.0".into();
        store.save_record(migrated).unwrap();
        let reloaded = store.load_record(&original.process.process_id).unwrap();
        assert_eq!(reloaded.process.definition_version, "2.0.0");
        assert_eq!(reloaded.creation, original.creation);
    }

    #[test]
    fn creation_snapshot_shape_valid_wrong_process_is_refused() {
        let mut record = creation_record();
        record.creation.as_mut().unwrap().process_id = fixture_minter().mint_process_id();
        record.creation.as_ref().unwrap().validate().unwrap();
        assert!(matches!(
            record.validate_creation(),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
    }

    fn restore_original_record() -> RuntimeRecord {
        use base64::Engine as _;
        let mut record = creation_record();
        // Match the maintained runtime task encoding without changing its visibility.
        let parent =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&record.process.process_id);
        let task = serde_json::from_value(serde_json::json!({
            "taskId": format!("wos-task:{parent}:1:review"), "taskRef": "review",
            "status": "assigned", "assignedActor": "original-reviewer",
            "createdAt": record.process.created_at, "updatedAt": record.process.created_at
        }))
        .unwrap();
        record.process.active_tasks.push(task);
        let kernel = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": record.process.definition_url, "version": "1.0.0",
            "governance": { "taskCatalog": [{ "id": "review", "title": "Original Review", "pattern": "review" }] },
            "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic" } } }
        })).unwrap();
        record.creation = Some(ProcessCreationSnapshot::capture(&record.process, &kernel).unwrap());
        record
    }

    #[test]
    fn restore_record_preserves_creation_after_task_progress_and_version_migration() {
        let original = restore_original_record();
        let id = original.process.process_id.clone();
        let case = original.process.case_ledger_id.clone();
        let mut persisted = InMemoryStore::new();
        persisted.create_record(original.clone()).unwrap();
        let mut progressed = persisted.load_record(&id).unwrap();
        progressed.process.active_tasks.clear();
        progressed.process.definition_version = "2.0.0".into();
        progressed.process.case_state = serde_json::json!({ "approved": true });
        progressed
            .provenance_log
            .push(stamped_event("2026-01-01T00:00:00Z", "reviewed"));
        progressed.replay_entries.insert(
            ReplayKey {
                operation: ReplayOperation::PersistDraft,
                task_id: original.creation.as_ref().unwrap().tasks[0].task_id.clone(),
                actor_id: "original-reviewer".into(),
                token: "original-token".into(),
            },
            ReplayValue::Draft(PersistDraftResult {
                artifact_id: "original-artifact".into(),
            }),
        );
        persisted.save_record(progressed.clone()).unwrap();
        let aux = runtime_aux_from_json(&runtime_aux_to_json(&progressed)).unwrap();
        let restored = RuntimeRecord {
            process: serde_json::from_value(serde_json::to_value(&progressed.process).unwrap())
                .unwrap(),
            creation: aux.creation,
            provenance_log: progressed.provenance_log.clone(),
            step_results: aux.step_results,
            artifacts: aux.artifacts,
            replay_entries: aux.replay_entries,
        };
        let mut strict = InMemoryStore::new();
        assert!(matches!(
            strict.create_record(restored.clone()),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        assert!(strict.processes_for_case(&case).is_empty());
        let mut loaded = crate::InMemoryStore::new();
        loaded.restore_record(restored).unwrap();
        let current = loaded.load_record(&id).unwrap();
        assert_eq!(current.creation, original.creation);
        assert_eq!(
            serde_json::to_value(&current.process).unwrap(),
            serde_json::to_value(&progressed.process).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&current.provenance_log).unwrap(),
            serde_json::to_value(&progressed.provenance_log).unwrap()
        );
        assert_eq!(
            runtime_aux_to_json(&current),
            runtime_aux_to_json(&progressed)
        );
        assert_eq!(loaded.processes_for_case(&case), vec![id]);
        let mut changed = current.clone();
        changed.creation.as_mut().unwrap().tasks[0].catalog_title = Some("Replaced".into());
        assert!(matches!(
            loaded.save_record(changed),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut erased = current;
        erased.creation = None;
        assert!(matches!(
            loaded.save_record(erased),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
    }

    #[test]
    fn restore_record_refuses_corrupt_or_mismatched_evidence_without_index_changes() {
        let original = restore_original_record();
        let mut corrupt = original.clone();
        corrupt.creation.as_mut().unwrap().created_at = "invalid".into();
        let mut wrong_process = original.clone();
        wrong_process.creation.as_mut().unwrap().process_id = fixture_minter().mint_process_id();
        let mut wrong_case = original.clone();
        wrong_case.creation.as_mut().unwrap().case_ledger_id =
            fixture_minter().mint_case_ledger_id();
        wrong_case.creation.as_ref().unwrap().validate().unwrap();
        for invalid in [corrupt, wrong_process, wrong_case] {
            let id = invalid.process.process_id.clone();
            let case = invalid.process.case_ledger_id.clone();
            let mut store = InMemoryStore::new();
            assert!(matches!(
                store.restore_record(invalid),
                Err(StoreError::InvalidCreationEvidence(_))
            ));
            assert!(matches!(
                store.load_record(&id),
                Err(StoreError::NotFound(_))
            ));
            assert!(store.processes_for_case(&case).is_empty());
        }
    }

    #[test]
    fn restore_record_duplicate_preserves_record_and_case_index() {
        let original = restore_original_record();
        let id = original.process.process_id.clone();
        let case = original.process.case_ledger_id.clone();
        let mut store = InMemoryStore::new();
        store.restore_record(original.clone()).unwrap();
        let mut duplicate = original.clone();
        let other_case = fixture_minter().mint_case_ledger_id();
        duplicate.process.case_ledger_id = other_case.clone();
        duplicate.creation.as_mut().unwrap().case_ledger_id = other_case.clone();
        duplicate.process.case_state = serde_json::json!({ "replaced": true });
        assert!(
            matches!(store.restore_record(duplicate), Err(StoreError::AlreadyExists(found)) if found == id)
        );
        assert_eq!(store.processes_for_case(&case), vec![id.clone()]);
        assert!(store.processes_for_case(&other_case).is_empty());
        let current = store.load_record(&id).unwrap();
        assert_eq!(current.creation, original.creation);
        assert_eq!(current.process.case_state, original.process.case_state);
        let mut sibling = restore_original_record();
        sibling.process.case_ledger_id = case.clone();
        sibling.creation.as_mut().unwrap().case_ledger_id = case.clone();
        let sibling_id = sibling.process.process_id.clone();
        store.restore_record(sibling).unwrap();
        assert_eq!(store.processes_for_case(&case), vec![id, sibling_id]);
    }

    #[test]
    fn process_store_refuses_missing_creation_without_records_or_index_changes() {
        let original = restore_original_record();
        let id = original.process.process_id.clone();
        let case = original.process.case_ledger_id.clone();
        let mut missing = original.clone();
        missing.creation = None;
        let mut store = InMemoryStore::new();
        assert!(matches!(
            store.create_record(missing.clone()),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        missing.process.active_tasks.clear();
        assert!(matches!(
            store.restore_record(missing),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        assert!(matches!(
            store.load_record(&id),
            Err(StoreError::NotFound(_))
        ));
        assert!(store.processes_for_case(&case).is_empty());
        store.restore_record(original.clone()).unwrap();
        assert_eq!(store.load_record(&id).unwrap().creation, original.creation);
    }

    fn record_with(process_id: &str, case_ledger_id: &str) -> RuntimeRecord {
        let instance = WorkflowProcess {
            actor_bindings: None,
            process_id: process_id.to_string(),
            case_ledger_id: case_ledger_id.to_string(),
            tenant: stack_common_typeid::Tenant::of(process_id)
                .expect("test process ids are TypeIDs")
                .to_string(),
            definition_url: "urn:test:case-scoped-store".to_string(),
            definition_version: "1.0.0".to_string(),
            configuration: Vec::new(),
            case_state: serde_json::Value::Null,
            provenance_position: 0,
            next_task_sequence: 0,
            timers: Vec::new(),
            active_tasks: Vec::new(),
            history_store: Default::default(),
            compensation_logs: Default::default(),
            status: InstanceStatus::Active,
            stalled_since: None,
            decline_reason: None,
            voided_by: None,
            voided_at: None,
            expired_at: None,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            pending_callbacks: Default::default(),
            pending_outbound_events: Default::default(),
            next_deferred_callback_sequence: 0,
            created_at: "1970-01-01T00:00:00Z".to_string(),
            updated_at: "1970-01-01T00:00:00Z".to_string(),
            extensions: Default::default(),
        };
        let mut record = RuntimeRecord::new(instance);
        let kernel = serde_json::from_value(serde_json::json!({
            "$wosWorkflow": "1.0", "url": record.process.definition_url,
            "version": record.process.definition_version,
            "lifecycle": { "initialState": "open", "states": { "open": { "type": "atomic" } } }
        }))
        .unwrap();
        record.creation = Some(
            crate::intake::ProcessCreationSnapshot::capture(&record.process, &kernel).unwrap(),
        );
        record
    }

    fn stamped_event(timestamp: &str, event: &str) -> ProvenanceRecord {
        let mut record = ProvenanceRecord::state_transition(
            &fixture_minter(),
            "from",
            "to",
            event,
            Some("actor:test"),
        );
        record.timestamp = timestamp.to_string();
        record
    }

    fn stamped(timestamp: &str) -> ProvenanceRecord {
        stamped_event(timestamp, "evt")
    }

    /// Every record in a written log must carry the process's tenant.
    #[test]
    fn writes_refuse_records_outside_the_process_tenant() {
        let process_id = "acme_process_01hw7rm71vfay8vvw14d2pf2a0";
        let case_id = "acme_case_01hw7rm71vfay8vvw14d2pf2a1";
        let minted_in = |tenant: &str, timestamp: &str| {
            let mut record = stamped(timestamp);
            record.id = stack_common_typeid::TypeIdMinter::fixture(tenant).mint_provenance_id();
            record
        };

        // Creation and restore check the whole log.
        let mut foreign_at_creation = record_with(process_id, case_id);
        foreign_at_creation
            .provenance_log
            .push(minted_in("default", "2026-01-01T00:00:00Z"));
        assert!(matches!(
            InMemoryStore::new().create_record(foreign_at_creation.clone()),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
        assert!(matches!(
            InMemoryStore::new().restore_record(foreign_at_creation),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));

        // A stored tenant that disagrees with the ID prefix is corrupt data.
        let mut mismatched = record_with(process_id, case_id);
        mismatched.process.tenant = "beta".to_string();
        assert!(matches!(
            InMemoryStore::new().create_record(mismatched.clone()),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        assert!(matches!(
            InMemoryStore::new().restore_record(mismatched),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut store = InMemoryStore::new();
        store
            .create_record(record_with(process_id, case_id))
            .unwrap();
        let mut tampered = store.load_record(process_id).unwrap();
        tampered.process.tenant = "beta".to_string();
        assert!(matches!(
            store.save_record(tampered),
            Err(StoreError::InvalidCreationEvidence(_))
        ));
        let mut current = store.load_record(process_id).unwrap();
        current
            .provenance_log
            .push(minted_in("acme", "2026-01-02T00:00:00Z"));
        store.save_record(current).unwrap();

        // A record minted for another tenant is refused on save.
        let mut foreign = store.load_record(process_id).unwrap();
        foreign
            .provenance_log
            .push(minted_in("beta", "2026-01-03T00:00:00Z"));
        let error = store.save_record(foreign).unwrap_err();
        assert!(
            matches!(&error, StoreError::InvalidProvenanceEvidence(message)
                if message.contains("tenant `acme`")),
            "{error:?}"
        );

        // So is a record with no TypeID shape.
        let mut shapeless = store.load_record(process_id).unwrap();
        let mut bad = stamped("2026-01-04T00:00:00Z");
        bad.id = "prov-1".to_string();
        shapeless.provenance_log.push(bad);
        assert!(matches!(
            store.save_record(shapeless),
            Err(StoreError::InvalidProvenanceEvidence(_))
        ));
    }

    #[test]
    fn provenance_for_case_merges_records_from_multiple_processes_in_time_order() {
        let mut store = InMemoryStore::new();
        let case_ledger_id = "acme_case_01hw7rm71vfay8vvw14d2pf206";
        let process_a = "acme_process_01hw7rm71vfay8vvw14d2pf20b";
        let process_b = "acme_process_01hw7rm71vfay8vvw14d2pf20c";

        store
            .create_record(record_with(process_a, case_ledger_id))
            .unwrap();
        store
            .create_record(record_with(process_b, case_ledger_id))
            .unwrap();

        // Interleave timestamps across both processes — t1 (a), t2 (b), t3 (a), t4 (b).
        let mut record_a = store.load_record(process_a).unwrap();
        record_a
            .provenance_log
            .push(stamped("2026-05-12T10:00:01Z"));
        record_a
            .provenance_log
            .push(stamped("2026-05-12T10:00:03Z"));
        store.save_record(record_a).unwrap();

        let mut record_b = store.load_record(process_b).unwrap();
        record_b
            .provenance_log
            .push(stamped("2026-05-12T10:00:02Z"));
        record_b
            .provenance_log
            .push(stamped("2026-05-12T10:00:04Z"));
        store.save_record(record_b).unwrap();

        let merged = store.provenance_for_case(case_ledger_id);
        let timestamps: Vec<&str> = merged.iter().map(|r| r.timestamp.as_str()).collect();
        assert_eq!(
            timestamps,
            vec![
                "2026-05-12T10:00:01Z",
                "2026-05-12T10:00:02Z",
                "2026-05-12T10:00:03Z",
                "2026-05-12T10:00:04Z",
            ],
            "merged provenance MUST be in timestamp order across all processes bound to the case ledger",
        );

        let mut process_ids = store.processes_for_case(case_ledger_id);
        process_ids.sort();
        assert_eq!(
            process_ids,
            vec![process_a.to_string(), process_b.to_string()]
        );
    }

    #[test]
    fn provenance_for_case_preserves_encounter_order_for_identical_timestamps() {
        let mut store = InMemoryStore::new();
        let case_ledger_id = "acme_case_01hw7rm71vfay8vvw14d2pf209";
        let process_a = "acme_process_01hw7rm71vfay8vvw14d2pf20b";
        let process_b = "acme_process_01hw7rm71vfay8vvw14d2pf20c";
        let timestamp = "2026-05-12T10:00:00Z";

        // Create B before A to prove the tie-break is encounter order, not
        // process-id lexical ordering.
        store
            .create_record(record_with(process_b, case_ledger_id))
            .unwrap();
        store
            .create_record(record_with(process_a, case_ledger_id))
            .unwrap();

        let mut record_a = store.load_record(process_a).unwrap();
        record_a.provenance_log.push(stamped_event(timestamp, "a1"));
        record_a.provenance_log.push(stamped_event(timestamp, "a2"));
        store.save_record(record_a).unwrap();

        let mut record_b = store.load_record(process_b).unwrap();
        record_b.provenance_log.push(stamped_event(timestamp, "b1"));
        record_b.provenance_log.push(stamped_event(timestamp, "b2"));
        store.save_record(record_b).unwrap();

        let merged = store.provenance_for_case(case_ledger_id);
        let events: Vec<_> = merged
            .iter()
            .map(|record| {
                record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("transitionEvent"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            })
            .collect();
        assert_eq!(events, vec!["b1", "b2", "a1", "a2"]);
    }

    #[test]
    fn provenance_for_case_returns_empty_for_unknown_case_ledger() {
        let mut store = InMemoryStore::new();
        store
            .create_record(record_with(
                "acme_process_01hw7rm71vfay8vvw14d2pf20b",
                "acme_case_01hw7rm71vfay8vvw14d2pf205",
            ))
            .unwrap();

        assert!(
            store
                .provenance_for_case("acme_case_01hw7rm71vfay8vvw14d2pf204")
                .is_empty(),
            "unknown case ledger MUST yield an empty Vec, not panic or NotFound",
        );
        assert!(
            store
                .processes_for_case("acme_case_01hw7rm71vfay8vvw14d2pf204")
                .is_empty()
        );
    }

    #[test]
    fn save_record_rejects_case_ledger_id_mutation_without_reindexing() {
        let mut store = InMemoryStore::new();
        let process_id = "acme_process_01hw7rm71vfay8vvw14d2pf20d";
        let original_case = "acme_case_01hw7rm71vfay8vvw14d2pf208";
        let mutated_case = "acme_case_01hw7rm71vfay8vvw14d2pf207";
        store
            .create_record(record_with(process_id, original_case))
            .unwrap();

        let mut record = store.load_record(process_id).unwrap();
        record.process.case_ledger_id = mutated_case.to_string();
        let err = store
            .save_record(record)
            .expect_err("case-ledger binding mutation must fail");

        assert!(
            matches!(err, StoreError::Failed(_)),
            "case-ledger binding mutation should fail closed, got {err:?}",
        );
        assert!(
            err.to_string().contains("immutable"),
            "unexpected error: {err}",
        );
        assert_eq!(
            store.processes_for_case(original_case),
            vec![process_id.to_string()]
        );
        assert!(store.processes_for_case(mutated_case).is_empty());
        assert_eq!(
            store
                .load_record(process_id)
                .unwrap()
                .process
                .case_ledger_id,
            original_case
        );
    }

    #[test]
    fn append_provenance_for_case_writes_to_bound_process() {
        let mut store = InMemoryStore::new();
        let case_ledger_id = "acme_case_01hw7rm71vfay8vvw14d2pf202";
        let process_id = "acme_process_01hw7rm71vfay8vvw14d2pf20f";
        store
            .create_record(record_with(process_id, case_ledger_id))
            .unwrap();

        store
            .append_provenance_for_case(case_ledger_id, process_id, stamped("2026-05-12T11:00:00Z"))
            .expect("case-scoped append succeeds when binding matches");

        let merged = store.provenance_for_case(case_ledger_id);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].timestamp, "2026-05-12T11:00:00Z");

        let record = store.load_record(process_id).unwrap();
        assert_eq!(
            record.process.provenance_position, 1,
            "position tracks log length"
        );
    }

    #[test]
    fn append_provenance_for_case_rejects_mismatched_binding() {
        let mut store = InMemoryStore::new();
        store
            .create_record(record_with(
                "acme_process_01hw7rm71vfay8vvw14d2pf20e",
                "acme_case_01hw7rm71vfay8vvw14d2pf203",
            ))
            .unwrap();

        let err = store
            .append_provenance_for_case(
                "acme_case_01hw7rm71vfay8vvw14d2pf20a",
                "acme_process_01hw7rm71vfay8vvw14d2pf20e",
                stamped("2026-05-12T11:00:00Z"),
            )
            .expect_err("mismatched case ledger MUST be refused");
        assert!(
            matches!(err, StoreError::NotFound(_)),
            "binding mismatch surfaces as NotFound, got {err:?}",
        );
    }
}
