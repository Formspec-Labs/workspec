// Rust guideline compliant 2026-02-21
//! Complete native workflow storage and atomic intent-driven edits.

use serde_json::{Map, Value, json};
use wos_core::{ActorKind, ImpactLevel, KernelDocument, StateKind, TransitionEvent};

use crate::{
    command::{AppliedCommand, Command, CommandResult},
    diagnostics::AuthoringDiagnostic,
    native_schema,
};

pub(crate) trait IWosProjectCore {
    fn undo(&mut self) -> Result<(), AuthoringDiagnostic>;
    fn redo(&mut self) -> Result<(), AuthoringDiagnostic>;
    fn native_snapshot(&self) -> Value;
    fn typed_snapshot(&self) -> Result<KernelDocument, AuthoringDiagnostic>;
    fn diagnostics(&self) -> &[AuthoringDiagnostic];
}

// Full source snapshots keep undo lossless. The existing interactive history
// bound prevents unbounded retention of complete authored documents.
const UNDO_DEPTH: usize = 100;

#[derive(Debug, Clone)]
struct HistoryEntry {
    applied: AppliedCommand,
    before: Value,
}

/// The complete JSON value is the only writable document authority.
#[derive(Debug)]
pub(crate) struct RawWosProject {
    doc: Value,
    history: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    diagnostics: Vec<AuthoringDiagnostic>,
}

fn error(path: impl Into<String>, message: impl Into<String>) -> AuthoringDiagnostic {
    AuthoringDiagnostic::error(path, message)
}

fn object_at<'a>(
    document: &'a mut Value,
    pointer: &str,
) -> Result<&'a mut Map<String, Value>, AuthoringDiagnostic> {
    document
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| error(pointer, "expected an existing native object"))
}

fn array_at<'a>(
    document: &'a mut Value,
    pointer: &str,
) -> Result<&'a mut Vec<Value>, AuthoringDiagnostic> {
    document
        .pointer_mut(pointer)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| error(pointer, "expected an existing native array"))
}

fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// Check a locator for the specific native state operation, not a general patch.
fn state_pointer(pointer: &str) -> Result<(), AuthoringDiagnostic> {
    let tokens: Vec<_> = pointer.split('/').skip(1).collect();
    if tokens.len() < 3 || tokens[0] != "lifecycle" || tokens[1] != "states" || tokens[2].is_empty()
    {
        return Err(error(pointer, "state path must identify a lifecycle state"));
    }
    let mut next = 3;
    while next < tokens.len() {
        let width = match tokens[next..] {
            ["states", id, ..] if !id.is_empty() => 2,
            ["regions", region, "states", id, ..] if !region.is_empty() && !id.is_empty() => 4,
            _ => {
                return Err(error(
                    pointer,
                    "state path must follow native states or region states",
                ));
            }
        };
        next += width;
    }
    Ok(())
}

impl RawWosProject {
    pub(crate) fn new(impact_level: ImpactLevel, title: impl Into<String>) -> Self {
        Self::from_native_document(json!({
            "$wosWorkflow":"1.0", "title":title.into(), "impactLevel":impact_level,
            "actors":[], "lifecycle":{"initialState":"", "states":{}, "milestones":{}},
            "contracts":{}, "decisionTables":[], "extensions":{}
        }))
        .expect("built-in authoring draft is an object")
    }

    pub(crate) fn from_document(document: KernelDocument) -> Self {
        Self::from_native_document(serde_json::to_value(document).expect("native model serializes"))
            .expect("native model serializes as an object")
    }

    pub(crate) fn from_native_document(document: Value) -> Result<Self, AuthoringDiagnostic> {
        if !document.is_object() {
            return Err(error("/", "workflow document must be a JSON object"));
        }
        Ok(Self {
            doc: document,
            history: Vec::new(),
            redo_stack: Vec::new(),
            diagnostics: Vec::new(),
        })
    }

    fn apply_add_state(
        &mut self,
        id: String,
        kind: StateKind,
        description: Option<String>,
        metadata: Option<Value>,
    ) -> CommandResult {
        let mut state = json!({"type":kind});
        if let Some(description) = description {
            state["description"] = json!(description);
        }
        if let Some(metadata) = metadata {
            state["extensions"] = json!({"x-meta":metadata});
        }
        let states = object_at(&mut self.doc, "/lifecycle/states")?;
        if states.contains_key(&id) {
            return Err(error(
                format!("/lifecycle/states/{}", escape(&id)),
                format!("state '{id}' already exists"),
            ));
        }
        states.insert(id.clone(), state);
        Ok(AppliedCommand::with_inverse(
            format!("AddState({id})"),
            Command::RemoveState { id },
        ))
    }

    fn apply_remove_state(&mut self, id: String) -> CommandResult {
        let states = object_at(&mut self.doc, "/lifecycle/states")?;
        let removed = states.remove(&id).ok_or_else(|| {
            error(
                format!("/lifecycle/states/{}", escape(&id)),
                format!("state '{id}' not found"),
            )
        })?;
        let mut count = removed
            .get("transitions")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        for state in states.values_mut() {
            if let Some(transitions) = state.get_mut("transitions").and_then(Value::as_array_mut) {
                let before = transitions.len();
                transitions.retain(|transition| {
                    transition.get("target").and_then(Value::as_str) != Some(id.as_str())
                });
                count += before - transitions.len();
            }
        }
        Ok(AppliedCommand::without_inverse(format!(
            "RemoveState({id}); removed {count} transitions"
        )))
    }

    fn apply_set_initial_state(&mut self, state_id: String) -> CommandResult {
        if !object_at(&mut self.doc, "/lifecycle/states")?.contains_key(&state_id) {
            return Err(error(
                "/lifecycle/initialState",
                format!("state '{state_id}' does not exist; add it before setting as initial"),
            ));
        }
        self.doc["lifecycle"]["initialState"] = json!(state_id);
        Ok(AppliedCommand::without_inverse(format!(
            "SetInitialState({state_id})"
        )))
    }

    fn apply_rename_state(&mut self, old_id: String, new_id: String) -> CommandResult {
        let states = object_at(&mut self.doc, "/lifecycle/states")?;
        if states.contains_key(&new_id) {
            return Err(error(
                format!("/lifecycle/states/{}", escape(&new_id)),
                format!("state '{new_id}' already exists"),
            ));
        }
        if !states.contains_key(&old_id) {
            return Err(error(
                format!("/lifecycle/states/{}", escape(&old_id)),
                format!("state '{old_id}' not found"),
            ));
        }
        let old = std::mem::take(states);
        for (id, value) in old {
            states.insert(if id == old_id { new_id.clone() } else { id }, value);
        }
        for state in states.values_mut() {
            if let Some(transitions) = state.get_mut("transitions").and_then(Value::as_array_mut) {
                for transition in transitions {
                    if transition.get("target").and_then(Value::as_str) == Some(old_id.as_str()) {
                        transition["target"] = json!(new_id);
                    }
                }
            }
        }
        if self.doc["lifecycle"]["initialState"] == old_id {
            self.doc["lifecycle"]["initialState"] = json!(new_id);
        }
        Ok(AppliedCommand::with_inverse(
            format!("RenameState({old_id} → {new_id})"),
            Command::RenameState {
                old_id: new_id,
                new_id: old_id,
            },
        ))
    }

    fn apply_add_transition(
        &mut self,
        from: String,
        to: String,
        guard: Option<String>,
        event: Option<String>,
        typed_event: Option<TransitionEvent>,
    ) -> CommandResult {
        let states = object_at(&mut self.doc, "/lifecycle/states")?;
        if !states.contains_key(&from) {
            return Err(error(
                format!("/lifecycle/states/{}", escape(&from)),
                format!("source state '{from}' does not exist"),
            ));
        }
        if !states.contains_key(&to) {
            return Err(error(
                format!("/lifecycle/states/{}", escape(&to)),
                format!("target state '{to}' does not exist"),
            ));
        }
        let legacy = event
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if typed_event.is_some() && legacy.is_some() {
            return Err(error(
                "/lifecycle/transitions",
                "use either event or typed event, not both",
            ));
        }
        let event = typed_event.or_else(|| legacy.map(TransitionEvent::from_authoring_trigger));
        let mut transition = json!({"target":to});
        if let Some(event) = event {
            transition["event"] = serde_json::to_value(event).expect("event serializes");
        }
        if let Some(guard) = guard {
            transition["guard"] = json!(guard);
        }
        let source = states
            .get_mut(&from)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| error("/lifecycle/states", "source state must be an object"))?;
        source
            .entry("transitions")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| error("/lifecycle/states", "transitions must be an array"))?
            .push(transition);
        Ok(AppliedCommand::without_inverse(format!(
            "AddTransition({from} → {to})"
        )))
    }

    fn apply_add_actor(&mut self, id: String, kind: ActorKind) -> CommandResult {
        let actors = array_at(&mut self.doc, "/actors")?;
        if actors
            .iter()
            .any(|actor| actor.get("id").and_then(Value::as_str) == Some(id.as_str()))
        {
            return Err(error(
                format!("/actors/{}", escape(&id)),
                format!("actor '{id}' already exists"),
            ));
        }
        actors.push(json!({"id":id,"type":kind}));
        Ok(AppliedCommand::without_inverse("AddActor"))
    }

    fn apply_remove_actor(&mut self, id: String) -> CommandResult {
        let actors = array_at(&mut self.doc, "/actors")?;
        let index = actors
            .iter()
            .position(|actor| actor.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| {
                error(
                    format!("/actors/{}", escape(&id)),
                    format!("actor '{id}' not found"),
                )
            })?;
        actors.remove(index);
        let mut referenced = false;
        if let Some(states) = self
            .doc
            .pointer_mut("/lifecycle/states")
            .and_then(Value::as_object_mut)
        {
            for state in states.values_mut() {
                for phase in ["onEntry", "onExit"] {
                    referenced |=
                        state
                            .get(phase)
                            .and_then(Value::as_array)
                            .is_some_and(|actions| {
                                actions.iter().any(|action| {
                                    action.get("assignTo").and_then(Value::as_str)
                                        == Some(id.as_str())
                                })
                            });
                }
                if let Some(transitions) = state.get("transitions").and_then(Value::as_array) {
                    referenced |= transitions.iter().any(|transition| {
                        transition
                            .get("actions")
                            .and_then(Value::as_array)
                            .is_some_and(|actions| {
                                actions.iter().any(|action| {
                                    action.get("assignTo").and_then(Value::as_str)
                                        == Some(id.as_str())
                                })
                            })
                    });
                }
            }
        }
        if referenced {
            self.diagnostics.push(AuthoringDiagnostic::warning(
                format!("/actors/{}", escape(&id)),
                format!("removed actor '{id}' remains referenced by an action"),
            ));
        }
        Ok(AppliedCommand::without_inverse(format!(
            "RemoveActor({id})"
        )))
    }

    fn apply_add_actor_extension(
        &mut self,
        actor_id: String,
        key: String,
        value: Value,
    ) -> CommandResult {
        if !key.starts_with("x-") {
            return Err(error(
                format!("/actors/{actor_id}/extensions/{key}"),
                "extension key must start with 'x-'",
            ));
        }
        let actor = array_at(&mut self.doc, "/actors")?
            .iter_mut()
            .find(|actor| actor.get("id").and_then(Value::as_str) == Some(actor_id.as_str()))
            .and_then(Value::as_object_mut)
            .ok_or_else(|| error(format!("/actors/{}", escape(&actor_id)), "actor not found"))?;
        actor
            .entry("extensions")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/actors", "extensions must be an object"))?
            .insert(key, value);
        Ok(AppliedCommand::without_inverse("AddActorExtension"))
    }

    fn apply_set_impact_level(&mut self, level: ImpactLevel) -> CommandResult {
        self.doc["impactLevel"] = json!(level);
        Ok(AppliedCommand::without_inverse(format!(
            "SetImpactLevel({level:?})"
        )))
    }

    fn apply_add_contract(
        &mut self,
        name: String,
        binding: String,
        reference: String,
    ) -> CommandResult {
        let contracts = self
            .doc
            .as_object_mut()
            .expect("document is an object")
            .entry("contracts")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/contracts", "contracts must be an object"))?;
        if contracts.contains_key(&name) {
            return Err(error(
                format!("/contracts/{}", escape(&name)),
                format!("contract '{name}' already exists"),
            ));
        }
        contracts.insert(name.clone(), json!({"binding":binding,"ref":reference}));
        Ok(AppliedCommand::without_inverse(format!(
            "AddContract({name})"
        )))
    }

    fn apply_set_contract_definition_version(
        &mut self,
        name: String,
        version: String,
    ) -> CommandResult {
        if version.trim().is_empty() {
            return Err(error(
                format!("/contracts/{name}/definitionVersion"),
                "contract Definition version must not be blank",
            ));
        }
        let contract = object_at(&mut self.doc, &format!("/contracts/{}", escape(&name)))?;
        contract.insert("definitionVersion".into(), json!(version));
        Ok(AppliedCommand::without_inverse(
            "SetContractDefinitionVersion",
        ))
    }

    fn apply_add_milestone(&mut self, id: String, condition: String) -> CommandResult {
        let lifecycle = object_at(&mut self.doc, "/lifecycle")?;
        let milestones = lifecycle
            .entry("milestones")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/lifecycle/milestones", "milestones must be an object"))?;
        if milestones.contains_key(&id) {
            return Err(error(
                format!("/lifecycle/milestones/{}", escape(&id)),
                format!("milestone '{id}' already exists"),
            ));
        }
        milestones.insert(id.clone(), json!({"condition":condition}));
        Ok(AppliedCommand::without_inverse(format!(
            "AddMilestone({id})"
        )))
    }

    fn apply_remove_milestone(&mut self, id: String) -> CommandResult {
        object_at(&mut self.doc, "/lifecycle/milestones")?
            .remove(&id)
            .ok_or_else(|| {
                error(
                    format!("/lifecycle/milestones/{}", escape(&id)),
                    format!("milestone '{id}' not found"),
                )
            })?;
        Ok(AppliedCommand::without_inverse(format!(
            "RemoveMilestone({id})"
        )))
    }

    fn apply_set_timer(&mut self, timer_id: String, duration: String) -> CommandResult {
        let extensions = self
            .doc
            .as_object_mut()
            .expect("document is an object")
            .entry("extensions")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/extensions", "extensions must be an object"))?;
        let timers = extensions
            .entry("x-wos-timers")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| {
                error(
                    "/extensions/x-wos-timers",
                    "timer extension must be an object",
                )
            })?;
        timers.insert(timer_id.clone(), json!({"duration":duration}));
        Ok(AppliedCommand::without_inverse(format!(
            "SetTimer({timer_id})"
        )))
    }

    fn apply_add_extension_key(&mut self, key: String, value: Value) -> CommandResult {
        if !key.starts_with("x-") {
            return Err(error(
                format!("/extensions/{key}"),
                "extension key must start with 'x-'",
            ));
        }
        self.doc
            .as_object_mut()
            .expect("document is an object")
            .entry("extensions")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/extensions", "extensions must be an object"))?
            .insert(key.clone(), value);
        Ok(AppliedCommand::without_inverse(format!(
            "AddExtensionKey({key})"
        )))
    }

    fn apply_set_native_fields(&mut self, fields: Map<String, Value>) -> CommandResult {
        for (field, value) in &fields {
            native_schema::validate_workflow_field(field, value)?;
        }
        self.doc
            .as_object_mut()
            .expect("document is an object")
            .extend(fields);
        Ok(AppliedCommand::without_inverse("SetNativeFields"))
    }

    fn apply_put_case_field(&mut self, name: String, definition: Value) -> CommandResult {
        native_schema::validate_fragment("/$defs/FieldDeclaration", &definition)?;
        let case = self
            .doc
            .as_object_mut()
            .expect("document is an object")
            .entry("caseFile")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/caseFile", "case file must be an object"))?;
        if case.contains_key("contractRef") {
            return Err(error(
                "/caseFile",
                "inline fields cannot be added to a contract-referenced case file",
            ));
        }
        case.entry("fields")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/caseFile/fields", "fields must be an object"))?
            .insert(name, definition);
        Ok(AppliedCommand::without_inverse("PutCaseField"))
    }

    fn apply_put_contract(&mut self, name: String, contract: Value) -> CommandResult {
        native_schema::validate_fragment("/$defs/Contracts/additionalProperties", &contract)?;
        self.doc
            .as_object_mut()
            .expect("document is an object")
            .entry("contracts")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(|| error("/contracts", "contracts must be an object"))?
            .insert(name, contract);
        Ok(AppliedCommand::without_inverse("PutContract"))
    }

    fn apply_put_state(&mut self, pointer: String, state: Value) -> CommandResult {
        state_pointer(&pointer)?;
        native_schema::validate_fragment("/$defs/State", &state)?;
        let (parent, encoded_id) = pointer.rsplit_once('/').expect("state path checked");
        let id = encoded_id.replace("~1", "/").replace("~0", "~");
        object_at(&mut self.doc, parent)?.insert(id, state);
        Ok(AppliedCommand::without_inverse(format!(
            "PutState({pointer})"
        )))
    }

    fn apply_append_state_action(
        &mut self,
        pointer: String,
        phase: String,
        action: Value,
    ) -> CommandResult {
        state_pointer(&pointer)?;
        if !native_schema::state_action_phases()
            .iter()
            .any(|allowed| *allowed == phase)
        {
            return Err(error(&pointer, "unsupported state action phase"));
        }
        native_schema::validate_fragment("/$defs/LifecycleAction", &action)?;
        object_at(&mut self.doc, &pointer)?
            .entry(phase)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| error(&pointer, "state actions must be an array"))?
            .push(action);
        Ok(AppliedCommand::without_inverse("AppendStateAction"))
    }

    fn apply_put_agent(&mut self, agent: Value) -> CommandResult {
        native_schema::validate_fragment("/$defs/AgentDeclaration", &agent)?;
        let id = agent
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| error("/agents", "agent id is required"))?;
        let agents = self
            .doc
            .as_object_mut()
            .expect("document is an object")
            .entry("agents")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| error("/agents", "agents must be an array"))?;
        if let Some(index) = agents
            .iter()
            .position(|existing| existing.get("id").and_then(Value::as_str) == Some(id))
        {
            agents[index] = agent;
        } else {
            agents.push(agent);
        }
        Ok(AppliedCommand::without_inverse("PutAgent"))
    }

    fn apply_remove_agent(&mut self, id: String) -> CommandResult {
        let agents = array_at(&mut self.doc, "/agents")?;
        let index = agents
            .iter()
            .position(|agent| agent.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| error("/agents", "agent not found"))?;
        agents.remove(index);
        Ok(AppliedCommand::without_inverse("RemoveAgent"))
    }

    fn apply_add_agent_constraint(&mut self, id: String, constraint: Value) -> CommandResult {
        native_schema::validate_fragment("/$defs/DeonticConstraint", &constraint)?;
        let agent = array_at(&mut self.doc, "/agents")?
            .iter_mut()
            .find(|agent| agent.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .and_then(Value::as_object_mut)
            .ok_or_else(|| error("/agents", "agent not found"))?;
        agent
            .entry("deonticConstraints")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| error("/agents", "deontic constraints must be an array"))?
            .push(constraint);
        Ok(AppliedCommand::without_inverse("AddAgentConstraint"))
    }

    pub(crate) fn dispatch(&mut self, command: Command) -> CommandResult {
        let before = self.doc.clone();
        let diagnostic_count = self.diagnostics.len();
        let result = match command {
            Command::AddState {
                id,
                kind,
                description,
                metadata,
            } => self.apply_add_state(id, kind, description, metadata),
            Command::RemoveState { id } => self.apply_remove_state(id),
            Command::SetInitialState { state_id } => self.apply_set_initial_state(state_id),
            Command::RenameState { old_id, new_id } => self.apply_rename_state(old_id, new_id),
            Command::AddTransition {
                from_state,
                to_state,
                guard,
                event,
                event_typed,
            } => self.apply_add_transition(from_state, to_state, guard, event, event_typed),
            Command::AddActor { id, kind } => self.apply_add_actor(id, kind),
            Command::RemoveActor { id } => self.apply_remove_actor(id),
            Command::AddActorExtension {
                actor_id,
                key,
                value,
            } => self.apply_add_actor_extension(actor_id, key, value),
            Command::SetImpactLevel { level } => self.apply_set_impact_level(level),
            Command::AddContract {
                name,
                binding,
                ref_uri,
            } => self.apply_add_contract(name, binding, ref_uri),
            Command::SetContractDefinitionVersion {
                name,
                definition_version,
            } => self.apply_set_contract_definition_version(name, definition_version),
            Command::AddMilestone {
                milestone_id,
                condition,
            } => self.apply_add_milestone(milestone_id, condition),
            Command::RemoveMilestone { milestone_id } => self.apply_remove_milestone(milestone_id),
            Command::SetTimer { timer_id, duration } => self.apply_set_timer(timer_id, duration),
            Command::AddExtensionKey { key, value } => self.apply_add_extension_key(key, value),
            Command::SetNativeFields { fields } => self.apply_set_native_fields(fields),
            Command::PutCaseField { name, definition } => {
                self.apply_put_case_field(name, definition)
            }
            Command::PutContract { name, contract } => self.apply_put_contract(name, contract),
            Command::PutState { pointer, state } => self.apply_put_state(pointer, state),
            Command::AppendStateAction {
                pointer,
                phase,
                action,
            } => self.apply_append_state_action(pointer, phase, action),
            Command::PutAgent { agent } => self.apply_put_agent(agent),
            Command::RemoveAgent { id } => self.apply_remove_agent(id),
            Command::AddAgentConstraint { id, constraint } => {
                self.apply_add_agent_constraint(id, constraint)
            }
            Command::AddActorDeontic { .. }
            | Command::AddDueProcessPath { .. }
            | Command::AddAssertionGate { .. }
            | Command::AddDeonticConstraint { .. } => Err(error(
                "/",
                "obsolete extension operation is unsupported; author native governance or agent constraints",
            )),
        };
        if let Ok(applied) = &result {
            self.history.push(HistoryEntry {
                applied: applied.clone(),
                before,
            });
            if self.history.len() > UNDO_DEPTH {
                self.history.remove(0);
            }
            self.redo_stack.clear();
        } else {
            self.doc = before;
            self.diagnostics.truncate(diagnostic_count);
        }
        result
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

impl IWosProjectCore for RawWosProject {
    fn undo(&mut self) -> Result<(), AuthoringDiagnostic> {
        let entry = self
            .history
            .pop()
            .ok_or_else(|| error("/", "cannot undo: history is empty"))?;
        let current = std::mem::replace(&mut self.doc, entry.before);
        self.redo_stack.push(HistoryEntry {
            applied: entry.applied,
            before: current,
        });
        Ok(())
    }
    fn redo(&mut self) -> Result<(), AuthoringDiagnostic> {
        let entry = self
            .redo_stack
            .pop()
            .ok_or_else(|| error("/", "cannot redo: redo stack is empty"))?;
        let current = std::mem::replace(&mut self.doc, entry.before);
        self.history.push(HistoryEntry {
            applied: entry.applied,
            before: current,
        });
        Ok(())
    }
    fn native_snapshot(&self) -> Value {
        self.doc.clone()
    }
    fn typed_snapshot(&self) -> Result<KernelDocument, AuthoringDiagnostic> {
        serde_json::from_value(self.doc.clone()).map_err(|cause| {
            error(
                "/",
                format!("native document has no supported runtime typed view: {cause}"),
            )
        })
    }
    fn diagnostics(&self) -> &[AuthoringDiagnostic] {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Severity;
    use wos_core::model::decision_table::Guard;

    fn make_project() -> RawWosProject {
        RawWosProject::new(ImpactLevel::Operational, "Test Project")
    }

    // ── AddState ──────────────────────────────────────────────────────────

    /// Dispatching AddState inserts the state into the document snapshot.
    #[test]
    fn add_state_appears_in_snapshot() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .expect("AddState must succeed on an empty project");

        let snap = p.typed_snapshot().unwrap();
        assert!(
            snap.lifecycle.states.contains_key("draft"),
            "snapshot must contain the newly added state"
        );
    }

    /// A second AddState with the same ID must fail.
    #[test]
    fn add_state_duplicate_returns_error() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .expect("first AddState must succeed");

        let err = p
            .dispatch(Command::AddState {
                id: "draft".into(),
                kind: StateKind::Compound,
                description: None,
                metadata: None,
            })
            .expect_err("duplicate AddState must return an error diagnostic");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("already exists"),
            "error message must mention 'already exists'"
        );
    }

    // ── AddTransition ──────────────────────────────────────────────────────

    /// Dispatching AddTransition inserts the transition into the source state.
    #[test]
    fn add_transition_appears_in_snapshot() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "submitted".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddState {
            id: "approved".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        p.dispatch(Command::AddTransition {
            from_state: "submitted".into(),
            to_state: "approved".into(),
            guard: Some("caseFile.amount <= 50000".into()),
            event: Some("approve".into()),
            event_typed: None,
        })
        .expect("AddTransition must succeed when both states exist");

        let snap = p.typed_snapshot().unwrap();
        let transitions = &snap.lifecycle.states["submitted"].transitions;
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].target, "approved");
        assert_eq!(
            transitions[0]
                .event
                .as_ref()
                .map(|e| e.runtime_dispatch_label())
                .as_deref(),
            Some("approve")
        );
        assert_eq!(
            transitions[0].guard.as_ref().and_then(Guard::as_fel_str),
            Some("caseFile.amount <= 50000")
        );
    }

    /// AddTransition referencing an unknown source state must fail.
    #[test]
    fn add_transition_unknown_source_returns_error() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "approved".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddTransition {
                from_state: "nonexistent".into(),
                to_state: "approved".into(),
                guard: None,
                event: Some("approve".into()),
                event_typed: None,
            })
            .expect_err("AddTransition with unknown source must return an error");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("does not exist"),
            "error must mention 'does not exist'"
        );
    }

    /// AddTransition referencing an unknown target state must fail.
    #[test]
    fn add_transition_unknown_target_returns_error() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "submitted".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddTransition {
                from_state: "submitted".into(),
                to_state: "nonexistent".into(),
                guard: None,
                event: Some("approve".into()),
                event_typed: None,
            })
            .expect_err("AddTransition with unknown target must return an error");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("does not exist"),
            "error must mention 'does not exist'"
        );
    }

    /// AddTransition rejects legacy `event` together with `event_typed`.
    #[test]
    fn add_transition_legacy_and_typed_mutually_exclusive() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "submitted".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddState {
            id: "b".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddTransition {
                from_state: "submitted".into(),
                to_state: "b".into(),
                guard: None,
                event: Some("go".into()),
                event_typed: Some(TransitionEvent::Message {
                    name: "go".into(),
                    correlation_key: None,
                    data: None,
                }),
            })
            .expect_err("both event sources must be rejected");

        assert_eq!(err.severity, Severity::Error);
        assert!(err.message.contains("not both"));
    }

    /// Typed AddTransition persists the TransitionEvent shape on the snapshot.
    #[test]
    fn add_transition_typed_persists_message_metadata() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "submitted".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddState {
            id: "b".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        p.dispatch(Command::AddTransition {
            from_state: "submitted".into(),
            to_state: "b".into(),
            guard: None,
            event: None,
            event_typed: Some(TransitionEvent::Message {
                name: "notify".into(),
                correlation_key: Some("k1".into()),
                data: None,
            }),
        })
        .expect("typed AddTransition must succeed");

        let t = &p.typed_snapshot().unwrap().lifecycle.states["submitted"].transitions[0];
        assert!(
            matches!(
                &t.event,
                Some(TransitionEvent::Message {
                    name,
                    correlation_key: Some(ck),
                    ..
                }) if name == "notify" && ck == "k1"
            ),
            "unexpected transition event: {:?}",
            t.event
        );
    }

    // ── new() / snapshot() ────────────────────────────────────────────────

    /// A freshly constructed project has an empty state map.
    #[test]
    fn new_project_has_empty_lifecycle() {
        let p = make_project();
        let snap = p.typed_snapshot().unwrap();
        assert_eq!(snap.wos_workflow, "1.0");
        assert!(snap.lifecycle.states.is_empty());
    }

    // ── undo / redo ───────────────────────────────────────────────────────

    /// Undo on an empty history is an error (nothing to revert).
    #[test]
    fn undo_on_empty_history_errors() {
        let mut p = make_project();
        assert!(!p.can_undo());
        let result = p.undo();
        assert!(result.is_err());
    }

    /// Redo on an empty redo stack is an error.
    #[test]
    fn redo_on_empty_stack_errors() {
        let mut p = make_project();
        assert!(!p.can_redo());
        let result = p.redo();
        assert!(result.is_err());
    }

    /// Undo reverses AddState.
    #[test]
    fn undo_reverses_add_state() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        assert!(
            p.typed_snapshot()
                .unwrap()
                .lifecycle
                .states
                .contains_key("draft")
        );

        p.undo().expect("undo must succeed after a dispatch");
        assert!(
            !p.typed_snapshot()
                .unwrap()
                .lifecycle
                .states
                .contains_key("draft")
        );
        assert!(p.can_redo());
    }

    /// Redo re-applies the undone command.
    #[test]
    fn redo_reapplies_undone_add_state() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.undo().unwrap();
        p.redo().expect("redo must succeed after an undo");
        assert!(
            p.typed_snapshot()
                .unwrap()
                .lifecycle
                .states
                .contains_key("draft")
        );
    }

    /// A new forward dispatch clears the redo stack (no split-branch replays).
    #[test]
    fn forward_dispatch_clears_redo_stack() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "a".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.undo().unwrap();
        assert!(p.can_redo());

        // Forward motion on a different state id discards the redo branch.
        p.dispatch(Command::AddState {
            id: "b".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        assert!(!p.can_redo());
    }

    /// Property test: apply any sequence, undo all, state equals initial.
    #[test]
    fn undo_all_returns_to_initial_state() {
        let mut p = make_project();
        let initial = p.typed_snapshot().unwrap();

        // Diverse batch covering multiple handler paths.
        p.dispatch(Command::AddState {
            id: "s1".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddActor {
            id: "a1".into(),
            kind: ActorKind::Human,
        })
        .unwrap();
        p.dispatch(Command::SetImpactLevel {
            level: ImpactLevel::RightsImpacting,
        })
        .unwrap();
        p.dispatch(Command::AddContract {
            name: "c1".into(),
            binding: "formspec".into(),
            ref_uri: "urn:x:1".into(),
        })
        .unwrap();
        p.dispatch(Command::AddMilestone {
            milestone_id: "m1".into(),
            condition: "true".into(),
        })
        .unwrap();

        while p.can_undo() {
            p.undo().unwrap();
        }

        let final_snap = p.typed_snapshot().unwrap();
        assert_eq!(final_snap.actors.len(), initial.actors.len());
        assert_eq!(
            final_snap.lifecycle.states.len(),
            initial.lifecycle.states.len()
        );
        assert_eq!(final_snap.impact_level, initial.impact_level);
        assert_eq!(final_snap.contracts.len(), initial.contracts.len());
        assert_eq!(
            final_snap.lifecycle.milestones.len(),
            initial.lifecycle.milestones.len()
        );
    }

    /// Property test: undo then redo equals the post-apply state.
    #[test]
    fn undo_then_redo_reproduces_post_apply_state() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "s1".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddActor {
            id: "a1".into(),
            kind: ActorKind::Human,
        })
        .unwrap();
        let after_apply_states = p.typed_snapshot().unwrap().lifecycle.states.len();
        let after_apply_actors = p.typed_snapshot().unwrap().actors.len();

        p.undo().unwrap();
        p.undo().unwrap();
        p.redo().unwrap();
        p.redo().unwrap();

        let snap = p.typed_snapshot().unwrap();
        assert_eq!(snap.lifecycle.states.len(), after_apply_states);
        assert_eq!(snap.actors.len(), after_apply_actors);
    }

    /// Failed dispatches do not touch the history stack.
    #[test]
    fn failed_dispatch_leaves_history_untouched() {
        let mut p = make_project();
        let _ = p
            .dispatch(Command::AddState {
                id: "s1".into(),
                kind: StateKind::Atomic,
                description: None,
                metadata: None,
            })
            .unwrap();
        let _ = p.dispatch(Command::AddState {
            // Duplicate id — must fail.
            id: "s1".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        });

        // Only the first (successful) dispatch counted toward history.
        p.undo().unwrap();
        assert!(p.typed_snapshot().unwrap().lifecycle.states.is_empty());
        assert!(!p.can_undo());
    }

    // ── AddActor / RemoveActor ────────────────────────────────────────────

    /// AddActor appends a human actor to the document.
    #[test]
    fn add_actor_human_appears_in_snapshot() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
        })
        .expect("AddActor must succeed on empty project");

        let snap = p.typed_snapshot().unwrap();
        assert_eq!(snap.actors.len(), 1);
        assert_eq!(snap.actors[0].id, "approver");
        assert_eq!(snap.actors[0].kind, ActorKind::Human);
    }

    /// AddActor for the system kind also works — kernel §S3 allows only
    /// `human | system`. AI agents route through `x-wos-ai.agents`.
    #[test]
    fn add_actor_system_appears_in_snapshot() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "procurement".into(),
            kind: ActorKind::System,
        })
        .unwrap();

        let snap = p.typed_snapshot().unwrap();
        assert_eq!(snap.actors[0].kind, ActorKind::System);
    }

    /// AddActor twice with the same id must fail.
    #[test]
    fn add_actor_duplicate_returns_error() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddActor {
                id: "approver".into(),
                kind: ActorKind::Human,
            })
            .expect_err("duplicate AddActor must fail");

        assert!(err.message.contains("already exists"));
    }

    /// RemoveActor drops the actor from the vector.
    #[test]
    fn remove_actor_drops_entry() {
        let mut p = make_project();
        p.dispatch(Command::AddActor {
            id: "approver".into(),
            kind: ActorKind::Human,
        })
        .unwrap();
        p.dispatch(Command::RemoveActor {
            id: "approver".into(),
        })
        .expect("RemoveActor must succeed");

        assert!(p.typed_snapshot().unwrap().actors.is_empty());
    }

    /// RemoveActor for an unknown id errors.
    #[test]
    fn remove_actor_unknown_returns_error() {
        let mut p = make_project();
        let err = p
            .dispatch(Command::RemoveActor { id: "ghost".into() })
            .expect_err("unknown actor must be rejected");
        assert!(err.message.contains("not found"));
    }

    // ── RemoveState self-loop counting ────────────────────────────────────

    /// A state with only a self-loop must report exactly 1 transition removed,
    /// not 2.  The self-loop is an outgoing transition and must not be
    /// double-counted as an inbound transition from "another" state.
    #[test]
    fn remove_state_self_loop_counts_once() {
        let mut p = make_project();

        p.dispatch(Command::AddState {
            id: "loop".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        // Add a self-loop: loop → loop.
        p.dispatch(Command::AddTransition {
            from_state: "loop".into(),
            to_state: "loop".into(),
            guard: None,
            event: Some("retry".into()),
            event_typed: None,
        })
        .unwrap();

        // Remove the state.  The label must mention "1 transition removed",
        // not "2 transitions removed".
        let applied = p
            .dispatch(Command::RemoveState { id: "loop".into() })
            .expect("RemoveState must succeed");

        assert!(
            applied.label.contains("1 transition"),
            "self-loop must be counted exactly once; label was: {:?}",
            applied.label
        );
    }

    // ── SetImpactLevel ────────────────────────────────────────────────────

    /// SetImpactLevel on a fresh project updates the top-level field from
    /// its baseline value.
    #[test]
    fn set_impact_level_updates_document() {
        let mut p = make_project();
        // Baseline is Operational (from `make_project`).
        p.dispatch(Command::SetImpactLevel {
            level: ImpactLevel::RightsImpacting,
        })
        .expect("SetImpactLevel must succeed");

        assert_eq!(
            p.typed_snapshot().unwrap().impact_level,
            Some(ImpactLevel::RightsImpacting)
        );
    }

    /// SetImpactLevel accepts all four kernel §S6 variants.
    #[test]
    fn set_impact_level_accepts_all_variants() {
        for level in [
            ImpactLevel::RightsImpacting,
            ImpactLevel::SafetyImpacting,
            ImpactLevel::Operational,
            ImpactLevel::Informational,
        ] {
            let mut p = make_project();
            p.dispatch(Command::SetImpactLevel { level })
                .expect("SetImpactLevel must succeed for every variant");
            assert_eq!(p.typed_snapshot().unwrap().impact_level, Some(level));
        }
    }

    // ── AddContract ───────────────────────────────────────────────────────

    /// AddContract inserts a named reference into the contracts map.
    #[test]
    fn add_contract_appears_in_snapshot() {
        let mut p = make_project();
        p.dispatch(Command::AddContract {
            name: "purchaseOrderForm".into(),
            binding: "formspec".into(),
            ref_uri: "urn:formspec:example.gov:po:1.0".into(),
        })
        .expect("AddContract must succeed");

        let snap = p.typed_snapshot().unwrap();
        let contract = snap
            .contracts
            .get("purchaseOrderForm")
            .expect("contract must exist");
        assert_eq!(contract.binding, "formspec");
        assert_eq!(contract.reference, "urn:formspec:example.gov:po:1.0");
    }

    /// Form pins survive authoring export without changing the workflow version.
    #[test]
    fn authored_contract_form_version_is_independent_and_blank_update_is_atomic() {
        let mut p = make_project();
        p.dispatch(Command::AddContract {
            name: "review".into(),
            binding: "formspec".into(),
            ref_uri: "urn:form:review".into(),
        })
        .unwrap();
        let workflow_version = p.typed_snapshot().unwrap().version.clone();
        p.dispatch(Command::SetContractDefinitionVersion {
            name: "review".into(),
            definition_version: "0.3.2".into(),
        })
        .unwrap();
        let serialized = serde_json::to_value(p.typed_snapshot().unwrap()).unwrap();
        let restored: KernelDocument = serde_json::from_value(serialized.clone()).unwrap();
        assert_eq!(
            restored.contracts["review"].definition_version.as_deref(),
            Some("0.3.2")
        );
        assert_eq!(restored.version, workflow_version);
        for (name, version) in [("review", " "), ("missing", "0.3.1")] {
            assert!(
                p.dispatch(Command::SetContractDefinitionVersion {
                    name: name.into(),
                    definition_version: version.into()
                })
                .is_err()
            );
            assert_eq!(
                serde_json::to_value(p.typed_snapshot().unwrap()).unwrap(),
                serialized
            );
        }
    }

    /// AddContract twice with the same name must fail.
    #[test]
    fn add_contract_duplicate_returns_error() {
        let mut p = make_project();
        p.dispatch(Command::AddContract {
            name: "po".into(),
            binding: "formspec".into(),
            ref_uri: "urn:x:1".into(),
        })
        .unwrap();

        let err = p
            .dispatch(Command::AddContract {
                name: "po".into(),
                binding: "json-schema".into(),
                ref_uri: "urn:x:2".into(),
            })
            .expect_err("duplicate contract name must be rejected");

        assert!(err.message.contains("already exists"));
    }

    // ── RenameState ───────────────────────────────────────────────────────

    /// RenameState swaps the id in the lifecycle map while preserving order.
    #[test]
    fn rename_state_updates_map_key() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "draft".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        p.dispatch(Command::RenameState {
            old_id: "draft".into(),
            new_id: "submitted".into(),
        })
        .expect("RenameState must succeed");

        let snap = p.typed_snapshot().unwrap();
        assert!(!snap.lifecycle.states.contains_key("draft"));
        assert!(snap.lifecycle.states.contains_key("submitted"));
    }

    /// RenameState repoints transitions that targeted the old id.
    #[test]
    fn rename_state_repoints_incoming_transitions() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "a".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddState {
            id: "b".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddTransition {
            from_state: "a".into(),
            to_state: "b".into(),
            guard: None,
            event: Some("go".into()),
            event_typed: None,
        })
        .unwrap();

        p.dispatch(Command::RenameState {
            old_id: "b".into(),
            new_id: "c".into(),
        })
        .unwrap();

        let transitions = &p.typed_snapshot().unwrap().lifecycle.states["a"].transitions;
        assert_eq!(transitions[0].target, "c");
    }

    /// RenameState updates lifecycle.initialState when it matches.
    #[test]
    fn rename_state_repoints_initial_state() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "start".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.doc["lifecycle"]["initialState"] = json!("start");

        p.dispatch(Command::RenameState {
            old_id: "start".into(),
            new_id: "begin".into(),
        })
        .unwrap();

        assert_eq!(p.typed_snapshot().unwrap().lifecycle.initial_state, "begin");
    }

    /// RenameState rejects an unknown old id.
    #[test]
    fn rename_state_unknown_old_id_errors() {
        let mut p = make_project();
        let err = p
            .dispatch(Command::RenameState {
                old_id: "ghost".into(),
                new_id: "phantom".into(),
            })
            .expect_err("unknown state must be rejected");
        assert!(err.message.contains("not found"));
    }

    /// RenameState rejects collisions with an existing state id.
    #[test]
    fn rename_state_collision_errors() {
        let mut p = make_project();
        p.dispatch(Command::AddState {
            id: "a".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();
        p.dispatch(Command::AddState {
            id: "b".into(),
            kind: StateKind::Atomic,
            description: None,
            metadata: None,
        })
        .unwrap();

        let err = p
            .dispatch(Command::RenameState {
                old_id: "a".into(),
                new_id: "b".into(),
            })
            .expect_err("collision must be rejected");
        assert!(err.message.contains("already exists"));
    }

    // ── AddMilestone / RemoveMilestone ────────────────────────────────────

    /// AddMilestone inserts a condition into lifecycle.milestones.
    #[test]
    fn add_milestone_appears_in_snapshot() {
        let mut p = make_project();
        p.dispatch(Command::AddMilestone {
            milestone_id: "approved".into(),
            condition: "caseFile.status == \"approved\"".into(),
        })
        .expect("AddMilestone must succeed");

        let ms = p
            .typed_snapshot()
            .unwrap()
            .lifecycle
            .milestones
            .get("approved")
            .expect("milestone must exist")
            .clone();
        assert_eq!(ms.condition, "caseFile.status == \"approved\"");
    }

    /// Duplicate milestone ids are rejected.
    #[test]
    fn add_milestone_duplicate_returns_error() {
        let mut p = make_project();
        p.dispatch(Command::AddMilestone {
            milestone_id: "m1".into(),
            condition: "a".into(),
        })
        .unwrap();
        let err = p
            .dispatch(Command::AddMilestone {
                milestone_id: "m1".into(),
                condition: "b".into(),
            })
            .expect_err("duplicate milestone must be rejected");
        assert!(err.message.contains("already exists"));
    }

    /// RemoveMilestone drops the entry.
    #[test]
    fn remove_milestone_drops_entry() {
        let mut p = make_project();
        p.dispatch(Command::AddMilestone {
            milestone_id: "m1".into(),
            condition: "c".into(),
        })
        .unwrap();
        p.dispatch(Command::RemoveMilestone {
            milestone_id: "m1".into(),
        })
        .expect("RemoveMilestone must succeed");
        assert!(p.typed_snapshot().unwrap().lifecycle.milestones.is_empty());
    }

    /// RemoveMilestone for an unknown id errors.
    #[test]
    fn remove_milestone_unknown_errors() {
        let mut p = make_project();
        let err = p
            .dispatch(Command::RemoveMilestone {
                milestone_id: "ghost".into(),
            })
            .expect_err("unknown milestone must be rejected");
        assert!(err.message.contains("not found"));
    }

    // ── SetTimer ──────────────────────────────────────────────────────────

    /// SetTimer writes the duration under x-wos-timers.<timer_id>.
    #[test]
    fn set_timer_writes_extension() {
        let mut p = make_project();
        p.dispatch(Command::SetTimer {
            timer_id: "approvalDeadline".into(),
            duration: "P7D".into(),
        })
        .expect("SetTimer must succeed");

        let ext = &p.typed_snapshot().unwrap().extensions["x-wos-timers"];
        assert_eq!(ext["approvalDeadline"]["duration"], "P7D");
    }

    /// SetTimer re-assigns an existing timer id without error.
    #[test]
    fn set_timer_overwrites_existing() {
        let mut p = make_project();
        p.dispatch(Command::SetTimer {
            timer_id: "t1".into(),
            duration: "P1D".into(),
        })
        .unwrap();
        p.dispatch(Command::SetTimer {
            timer_id: "t1".into(),
            duration: "P30D".into(),
        })
        .unwrap();

        let ext = &p.typed_snapshot().unwrap().extensions["x-wos-timers"];
        assert_eq!(ext["t1"]["duration"], "P30D");
    }

    // ── AddActorDeontic ───────────────────────────────────────────────────

    /// AddActorDeontic appends a constraint under `x-wos-ai.deonticConstraints`
    /// and creates the extension scaffolding lazily.
    #[test]
    fn add_extension_key_stores_value() {
        let mut p = make_project();
        p.dispatch(Command::AddExtensionKey {
            key: "x-custom-meta".into(),
            value: serde_json::json!({ "owner": "procurement", "tier": 1 }),
        })
        .expect("x-prefixed extension must be accepted");

        let snap = p.typed_snapshot().unwrap();
        assert_eq!(
            snap.extensions["x-custom-meta"]["owner"],
            serde_json::json!("procurement")
        );
        assert_eq!(snap.extensions["x-custom-meta"]["tier"], 1);
    }

    /// Re-setting an existing extension key overwrites the prior value.
    #[test]
    fn add_extension_key_overwrites_existing() {
        let mut p = make_project();
        p.dispatch(Command::AddExtensionKey {
            key: "x-flag".into(),
            value: serde_json::json!(false),
        })
        .unwrap();
        p.dispatch(Command::AddExtensionKey {
            key: "x-flag".into(),
            value: serde_json::json!(true),
        })
        .unwrap();

        assert_eq!(
            p.typed_snapshot().unwrap().extensions["x-flag"],
            serde_json::json!(true)
        );
    }

    /// UNDO_DEPTH eviction: beyond 100 commands, the oldest snapshot drops.
    /// An undo-to-exhaustion after 101 commands must surface exactly 100
    /// reverts before `can_undo()` reports empty. The earliest command's
    /// effects therefore become unreachable from the current doc — the
    /// cap is a memory guard, not a fidelity guarantee.
    #[test]
    fn undo_depth_cap_evicts_oldest_entry() {
        let mut p = make_project();
        let depth = UNDO_DEPTH;

        // Apply depth + 1 commands. Each AddState is distinct so the
        // underlying dispatch actually mutates and pushes a history entry.
        for i in 0..=depth {
            p.dispatch(Command::AddState {
                id: format!("s{i}"),
                kind: StateKind::Atomic,
                description: None,
                metadata: None,
            })
            .expect("dispatch AddState");
        }

        // History is capped at depth; the oldest pre-empty snapshot was
        // evicted when len exceeded UNDO_DEPTH.
        assert_eq!(p.history.len(), depth, "history capped at UNDO_DEPTH");
        assert!(
            p.typed_snapshot()
                .unwrap()
                .lifecycle
                .states
                .contains_key(&format!("s0")),
            "s0 is still in the document (only its history entry is dropped)"
        );

        // Undo should succeed exactly UNDO_DEPTH times, then stop.
        let mut undo_count = 0;
        while p.can_undo() {
            p.undo().expect("undo");
            undo_count += 1;
            // Guard against runaway loops if the cap logic ever regresses.
            assert!(undo_count <= depth + 1, "can_undo must stop at cap");
        }
        assert_eq!(
            undo_count, depth,
            "undo exhausts after exactly UNDO_DEPTH reverts"
        );

        // The earliest command (s0) cannot be undone — its pre-s0 snapshot
        // was evicted. Verifies the intended cap behaviour: s0 remains
        // in the document after full undo-exhaustion.
        assert!(
            p.typed_snapshot()
                .unwrap()
                .lifecycle
                .states
                .contains_key("s0"),
            "s0 survives full undo-exhaustion because its history entry was evicted"
        );
        // But all later states s1..=s100 are reverted.
        for i in 1..=depth {
            assert!(
                !p.typed_snapshot()
                    .unwrap()
                    .lifecycle
                    .states
                    .contains_key(&format!("s{i}")),
                "s{i} reverted by undo"
            );
        }
    }

    /// AddExtensionKey with a key that lacks the `x-` prefix is rejected before dispatch.
    #[test]
    fn extension_key_must_start_with_x_dash() {
        let mut p = make_project();

        let err = p
            .dispatch(Command::AddExtensionKey {
                key: "custom-meta".into(),
                value: serde_json::json!({}),
            })
            .expect_err("missing x- prefix must be rejected");

        assert_eq!(err.severity, Severity::Error);
        assert!(
            err.message.contains("must start with 'x-'"),
            "error must mention the x- prefix requirement"
        );
    }
}
