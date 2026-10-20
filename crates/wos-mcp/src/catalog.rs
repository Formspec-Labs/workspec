// Rust guideline compliant 2026-02-21
//! One declaration owns discovery, defaults, validation, dispatch and generated help.
use crate::{errors::ToolError, registry::ProjectRegistry, tools};
use serde_json::{Map, Value, json};
use std::{future::Future, pin::Pin, sync::LazyLock};
use wos_authoring::native_schema::{fragment_reference as native, workflow_schema};

type Handler = for<'a> fn(
    &'a mut ProjectRegistry,
    &'a str,
    Value,
) -> Pin<Box<dyn Future<Output = Result<Value, ToolError>> + Send + 'a>>;

struct Action {
    name: &'static str,
    description: &'static str,
    input: Value,
    handler: Handler,
    fields: &'static [&'static str],
}
/// An executable declaration. A handler is mandatory for every advertised action.
pub struct Tool {
    name: &'static str,
    description: &'static str,
    input: Value,
    output: Value,
    handler: Option<Handler>,
    actions: Vec<Action>,
    default_action: Option<&'static str>,
    read_only: bool,
    fields: &'static [&'static str],
}
macro_rules! handler {
    ($function:path) => {
        |registry, project, args| Box::pin($function(registry, project, args))
    };
}
fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn string() -> Value {
    json!({"type":"string","minLength":1})
}
fn number() -> Value {
    json!({"type":"integer","minimum":0})
}
fn array(items: Value) -> Value {
    json!({"type":"array","items":items})
}
fn strings() -> Value {
    array(json!({"type":"string"}))
}
fn scoped(properties: Value, required: &[&str]) -> Value {
    let mut properties = properties
        .as_object()
        .expect("declaration properties are objects")
        .clone();
    properties.insert("project_id".into(), string());
    let mut required = required.to_vec();
    required.push("project_id");
    object(Value::Object(properties), &required)
}
fn tool(
    name: &'static str,
    description: &'static str,
    input: Value,
    output: Value,
    handler: Handler,
    read_only: bool,
    fields: &'static [&'static str],
) -> Tool {
    Tool {
        name,
        description,
        input,
        output,
        handler: Some(handler),
        actions: vec![],
        default_action: None,
        read_only,
        fields,
    }
}
fn fields_schema(fields: &[&str]) -> Value {
    let properties: Map<String, Value> = fields
        .iter()
        .map(|field| ((*field).into(), native(&format!("/properties/{field}"))))
        .collect();
    let mut schema = object(Value::Object(properties), &[]);
    schema["minProperties"] = json!(1);
    schema
}
fn action(
    name: &'static str,
    description: &'static str,
    properties: Value,
    required: &[&str],
    handler: Handler,
    fields: &'static [&'static str],
) -> Action {
    let mut input = scoped(properties, required);
    input["properties"]["action"] = json!({"const":name});
    input["required"]
        .as_array_mut()
        .expect("required is an array")
        .push(json!("action"));
    Action {
        name,
        description,
        input,
        handler,
        fields,
    }
}

static TOOLS: LazyLock<Vec<Tool>> = LazyLock::new(|| {
    let mut result = vec![
        tool(
            "wos_ping",
            "Check the server is responding.",
            object(json!({}), &[]),
            object(json!({"pong":{"const":true}}), &["pong"]),
            handler!(tools::ping),
            true,
            &[],
        ),
        tool(
            "wos_create_kernel",
            "Create an editable draft. Complete required Workflow fields before validation or execution.",
            object(json!({}), &[]),
            object(
                json!({"project_id":string(),"draft":{"const":true}}),
                &["project_id", "draft"],
            ),
            handler!(tools::wos_create_kernel),
            false,
            &[],
        ),
        tool(
            "wos_load_document",
            "Import native Workflow JSON without reconstructing its fields; accepts drafts that pass the existing lint gate.",
            json!({"type":"object","allOf":[object(json!({"json":string(),"path":string()}),&[])],"oneOf":[{"required":["json"]},{"required":["path"]}]}),
            object(json!({"project_id":string()}), &["project_id"]),
            handler!(tools::wos_load_document),
            false,
            &[],
        ),
        tool(
            "wos_export_document",
            "Export the exact current native JSON values, including unmodeled authored content.",
            scoped(json!({}), &[]),
            object(json!({"document":{"type":"string"}}), &["document"]),
            handler!(tools::wos_export_document),
            true,
            &[],
        ),
        tool(
            "wos_describe_document",
            "Count top-level states, transitions, actors and native agents.",
            scoped(json!({}), &[]),
            object(
                json!({"state_count":number(),"transition_count":number(),"actor_count":number(),"impact_level":{"type":"string"},"ai_agent_count":number()}),
                &[
                    "state_count",
                    "transition_count",
                    "actor_count",
                    "impact_level",
                    "ai_agent_count",
                ],
            ),
            handler!(tools::wos_describe_document),
            true,
            &[],
        ),
        tool(
            "wos_add_state",
            "Add a top-level state; use wos_edit_workflow.state for nested or complete native states.",
            scoped(
                json!({"state_id":string(),"kind":{"allOf":[native("/$defs/State/properties/type")],"default":"atomic"},"label":{"type":"string"},"metadata":{"type":"object"}}),
                &["state_id"],
            ),
            object(json!({"state_id":string()}), &["state_id"]),
            handler!(tools::wos_add_state),
            false,
            &["lifecycle"],
        ),
        tool(
            "wos_add_transition",
            "Add a native transition. Choose a string trigger or a typed event.",
            scoped(
                json!({"from":string(),"to":string(),"trigger":{"type":"string"},"event":native("/$defs/TransitionEvent"),"guard":{"type":"string"}}),
                &["from", "to"],
            ),
            object(
                json!({"from":string(),"to":string(),"trigger":{"type":"string"},"event":{"anyOf":[native("/$defs/TransitionEvent"),{"type":"null"}]}}),
                &["from", "to", "trigger", "event"],
            ),
            handler!(tools::wos_add_transition),
            false,
            &["lifecycle"],
        ),
        tool(
            "wos_set_initial_state",
            "Choose an existing top-level initial state.",
            scoped(json!({"state_id":string()}), &["state_id"]),
            object(json!({"state_id":string()}), &["state_id"]),
            handler!(tools::wos_set_initial_state),
            false,
            &["lifecycle"],
        ),
        tool(
            "wos_remove_state",
            "Remove a top-level state and transitions targeting it.",
            scoped(json!({"state_id":string()}), &["state_id"]),
            object(
                json!({"state_id":string(),"transitions_removed":number()}),
                &["state_id", "transitions_removed"],
            ),
            handler!(tools::wos_remove_state),
            false,
            &["lifecycle"],
        ),
        tool(
            "wos_add_actor",
            "Declare a human, system or agent actor.",
            scoped(
                json!({"actor_id":string(),"kind":{"allOf":[native("/$defs/Actor/properties/type/oneOf/0")],"default":"human"}}),
                &["actor_id"],
            ),
            object(json!({"actor_id":string()}), &["actor_id"]),
            handler!(tools::wos_add_actor),
            false,
            &["actors"],
        ),
        tool(
            "wos_add_actor_extension",
            "Attach an x-prefixed extension to an existing actor.",
            scoped(
                json!({"actor_id":string(),"key":{"type":"string","pattern":"^x-"},"value":{}}),
                &["actor_id", "key", "value"],
            ),
            object(
                json!({"actor_id":string(),"key":string()}),
                &["actor_id", "key"],
            ),
            handler!(tools::wos_add_actor_extension),
            false,
            &["actors"],
        ),
        tool(
            "wos_set_impact_level",
            "Set the native Workflow impact classification.",
            scoped(
                json!({"level":native("/properties/impactLevel")}),
                &["level"],
            ),
            object(
                json!({"level":native("/properties/impactLevel")}),
                &["level"],
            ),
            handler!(tools::wos_set_impact_level),
            false,
            &["impactLevel"],
        ),
        tool(
            "wos_lint",
            "Run canonical static validation in explicit document/project scope, retaining native findings and incomplete coverage. Static success is not replay or readiness.",
            scoped(
                json!({"scope":{"enum":["document","project"],"default":"document"},"resolved_dependencies":{"type":"array","default":[],"items":{"type":"object","properties":{"role":{"enum":["definition","mapping","delivery","ontology-alignment"]},"url":string(),"version":string(),"document_json":{},"definition":{"type":"object","properties":{"url":string(),"version":string()},"required":["url","version"],"additionalProperties":false}},"required":["role","url","version","document_json"],"additionalProperties":false}}}),
                &[],
            ),
            object(
                json!({"diagnostics":array(json!({"$ref":"https://wos-spec.org/schemas/tooling/1.0#/$defs/lintDiagnostic__Root"})),"block_count":number(),"error_count":number(),"warning_count":number(),"info_count":number(),"validation":{"$ref":"https://wos-spec.org/schemas/tooling/1.0#/$defs/staticValidation__Root"}}),
                &[
                    "diagnostics",
                    "block_count",
                    "error_count",
                    "warning_count",
                    "info_count",
                    "validation",
                ],
            ),
            handler!(tools::wos_lint),
            true,
            &[],
        ),
        tool(
            "wos_preview_state_graph",
            "Render a Mermaid or DOT state graph; reports typed-view limitations explicitly.",
            scoped(
                json!({"format":{"type":"string","enum":["mermaid","dot"],"default":"mermaid"}}),
                &[],
            ),
            object(
                json!({"graph":{"type":"string"},"format":{"enum":["mermaid","dot"]}}),
                &["graph", "format"],
            ),
            handler!(tools::wos_preview_state_graph),
            true,
            &[],
        ),
        tool(
            "wos_search",
            "Search states, transitions and actors through the typed read view; navigate raw native rule values and locations on drafts.",
            scoped(
                json!({"kind":{"enum":["state","transition","actor","constraint"]},"query":{"type":"string"}}),
                &["kind", "query"],
            ),
            object(
                json!({"matches":array(json!({"type":"object"})),"kind":string(),"query":{"type":"string"}}),
                &["matches", "kind", "query"],
            ),
            handler!(tools::wos_search),
            true,
            &[],
        ),
        tool(
            "wos_list_projects",
            "List open project identifiers.",
            object(json!({}), &[]),
            object(
                json!({"projects":strings(),"count":number()}),
                &["projects", "count"],
            ),
            handler!(tools::wos_list_projects),
            true,
            &[],
        ),
        tool(
            "wos_close_project",
            "Close an open project.",
            scoped(json!({}), &[]),
            object(
                json!({"project_id":string(),"closed":{"const":true}}),
                &["project_id", "closed"],
            ),
            handler!(tools::wos_close_project),
            false,
            &[],
        ),
    ];
    let dependency = object(
        json!({"role":string(),"url":string(),"version":{"type":["string","null"]},"document_json":string(),"definition":{"type":"object","properties":{"url":string(),"version":string()},"required":["url","version"],"additionalProperties":false}}),
        &["role", "url", "document_json"],
    );
    let expectation = object(
        json!({"pointer":{"type":"string"},"expected":{}}),
        &["pointer", "expected"],
    );
    let replay_input = scoped(
        json!({"fixture_json":string(),"resolved_dependencies":{"type":"array","items":dependency,"default":[]},"clock_start_ms":{"type":"integer","minimum":0,"maximum":u64::MAX,"default":0},"required_observations":{"type":"array","items":{"enum":wos_conformance::studio_api::SUPPORTED_OBSERVATIONS},"default":[]},"expected_observations":{"type":"array","items":expectation,"default":[]}}),
        &["fixture_json"],
    );
    let replay_output = object(
        json!({"passed":{"type":"boolean"},"status":{"enum":["pass","fail","refused"]},"identity":string(),"inputs":array(json!({"type":"object"})),"runtimeIdentity":{"type":"object"},"execution":{"type":"object"},"failures":strings(),"refusals":strings(),"differences":array(json!({"type":"object"})),"attempts":array(json!({"type":"object"})),"finalSnapshot":{"type":["object","null"]},"provenance":{"type":["array","null"]},"trace":{"type":"object"},"initialization":{"type":"object"},"observationErrors":strings()}),
        &[
            "passed",
            "status",
            "identity",
            "inputs",
            "runtimeIdentity",
            "execution",
            "failures",
            "refusals",
            "differences",
            "attempts",
            "finalSnapshot",
            "provenance",
            "trace",
            "initialization",
            "observationErrors",
        ],
    );
    result.push(tool("wos_run_conformance","Replay the selected current native Workflow and explicit frozen dependencies. Fixture assertions cannot replace selected documents. Mutable base_dir paths are unsupported.",replay_input,replay_output,handler!(tools::wos_run_conformance),true,&[]));
    let metadata_fields: &'static [&'static str] = &[
        "$wosWorkflow",
        "wosVersionPin",
        "url",
        "version",
        "title",
        "description",
        "status",
        "impactLevel",
    ];
    let actions = vec![
        action(
            "metadata",
            "Set selected identity and descriptive fields atomically.",
            json!({"fields":fields_schema(metadata_fields)}),
            &["fields"],
            handler!(tools::product::metadata),
            metadata_fields,
        ),
        action(
            "case_field",
            "Add or replace one inline case field.",
            json!({"name":string(),"definition":native("/$defs/FieldDeclaration")}),
            &["name", "definition"],
            handler!(tools::product::case_field),
            &["caseFile"],
        ),
        action(
            "contract",
            "Add or replace one native contract, including Formspec definition/version references.",
            json!({"name":string(),"contract":native("/$defs/Contracts/additionalProperties")}),
            &["name", "contract"],
            handler!(tools::product::contract),
            &["contracts"],
        ),
        action(
            "state",
            "Add or replace a native state at /lifecycle/states/<id> or a nested states/regions locator.",
            json!({"pointer":string(),"state":native("/$defs/State")}),
            &["pointer", "state"],
            handler!(tools::product::state),
            &["lifecycle"],
        ),
        action(
            "state_action",
            "Append a native action, including createTask/taskRef, to a state phase.",
            json!({"pointer":string(),"phase":{"enum":wos_authoring::native_schema::state_action_phases()},"value":native("/$defs/LifecycleAction")}),
            &["pointer", "phase", "value"],
            handler!(tools::product::state_action),
            &["lifecycle"],
        ),
        action(
            "governance",
            "Replace native governance, including taskCatalog and reviewProtocols.",
            json!({"value":native("/properties/governance")}),
            &["value"],
            handler!(tools::product::governance),
            &["governance"],
        ),
        action(
            "agent",
            "Add or replace a complete native agent by id.",
            json!({"value":native("/$defs/AgentDeclaration")}),
            &["value"],
            handler!(tools::product::agent),
            &["agents"],
        ),
        action(
            "remove_agent",
            "Remove one native agent by id.",
            json!({"agent_id":string()}),
            &["agent_id"],
            handler!(tools::product::remove_agent),
            &["agents"],
        ),
        action(
            "agent_constraint",
            "Append a canonical native agent deontic constraint.",
            json!({"agent_id":string(),"constraint":native("/$defs/DeonticConstraint")}),
            &["agent_id", "constraint"],
            handler!(tools::product::agent_constraint),
            &["agents"],
        ),
        action(
            "ai_oversight",
            "Replace native aiOversight.",
            json!({"value":native("/properties/aiOversight")}),
            &["value"],
            handler!(tools::product::ai_oversight),
            &["aiOversight"],
        ),
    ];
    result.push(Tool{name:"wos_edit_workflow",description:"Apply one native authoring edit. Fields outside the requested edit retain their exact JSON values. Omitted action defaults to metadata.",input:Value::Null,output:object(json!({"applied":string()}),&["applied"]),handler:None,actions,default_action:Some("metadata"),read_only:false,fields:&[]});
    result
});

struct OfflineRetriever;
impl jsonschema::Retrieve for OfflineRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let selected = match uri.as_str() {
            "https://wos-spec.org/schemas/wos-reference-due-process.schema.json" => Some(
                include_str!("../../../schemas/wos-reference-due-process.schema.json"),
            ),
            "https://wos-spec.org/schemas/wos-reference-replay.schema.json" => Some(include_str!(
                "../../../schemas/wos-reference-replay.schema.json"
            )),
            "https://wos-spec.org/schemas/wos-workflow.schema.json"
            | "https://wos-spec.org/schemas/workflow/1.0" => {
                Some(include_str!("../../../schemas/wos-workflow.schema.json"))
            }
            _ => None,
        };
        if let Some(text) = selected {
            return Ok(serde_json::from_str(text)?);
        }
        Err(format!("schema resource is not distributed locally: {uri}").into())
    }
}
fn validator(schema: &Value) -> Result<jsonschema::Validator, ToolError> {
    let tooling: Value =
        serde_json::from_str(include_str!("../../../schemas/wos-tooling.schema.json"))
            .map_err(|error| ToolError::Internal(error.to_string()))?;
    let registry = jsonschema::Registry::new()
        .retriever(OfflineRetriever)
        .add(
            workflow_schema()["$id"].as_str().expect("owner id"),
            workflow_schema().clone(),
        )
        .map_err(|error| ToolError::Internal(error.to_string()))?
        .add(tooling["$id"].as_str().expect("owner id"), tooling.clone())
        .map_err(|error| ToolError::Internal(error.to_string()))?
        .prepare()
        .map_err(|error| ToolError::Internal(error.to_string()))?;
    jsonschema::options()
        .with_registry(&registry)
        .build(schema)
        .map_err(|error| ToolError::Internal(format!("catalog schema: {error}")))
}
fn validate(schema: &Value, value: &Value) -> Result<(), ToolError> {
    if let Some(error) = validator(schema)?.iter_errors(value).next() {
        return Err(ToolError::InvalidArguments(error.to_string()));
    }
    Ok(())
}
fn defaults(schema: &Value, args: &mut Value) {
    if let (Some(properties), Some(args)) = (schema["properties"].as_object(), args.as_object_mut())
    {
        for (name, property) in properties {
            if !args.contains_key(name) {
                if let Some(default) = property.get("default") {
                    args.insert(name.clone(), default.clone());
                }
            }
        }
    }
}
/// Find an executable declaration by its only registered name.
pub fn find(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|tool| tool.name == name)
}
impl Tool {
    fn selected(&self, args: &Value) -> Result<(&Value, Handler), ToolError> {
        if self.actions.is_empty() {
            return Ok((
                &self.input,
                self.handler.expect("single tools have handlers"),
            ));
        }
        let name = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action must be a string".into()))?;
        let action = self
            .actions
            .iter()
            .find(|action| action.name == name)
            .ok_or_else(|| ToolError::InvalidArguments(format!("unknown action: {name}")))?;
        Ok((&action.input, action.handler))
    }
    fn prepare_arguments(&self, mut args: Value) -> Result<Value, ToolError> {
        if let Some(default) = self.default_action {
            if let Some(args) = args.as_object_mut() {
                args.entry("action").or_insert(json!(default));
            }
        }
        let (input, _) = self.selected(&args)?;
        defaults(input, &mut args);
        validate(input, &args)?;
        Ok(args)
    }
    /// Normalize declared defaults, validate and execute the selected handler.
    pub async fn execute(
        &self,
        registry: &mut ProjectRegistry,
        project: &str,
        args: Value,
    ) -> Result<Value, ToolError> {
        let args = self.prepare_arguments(args)?;
        let (_, handler) = self.selected(&args)?;
        let result = handler(registry, project, args).await?;
        validate(&self.output, &result)
            .map_err(|error| ToolError::Internal(format!("handler output: {error}")))?;
        Ok(result)
    }
    fn descriptor(&self) -> Value {
        let input = if self.actions.is_empty() {
            self.input.clone()
        } else {
            json!({"type":"object","oneOf":self.actions.iter().map(|action| {
            let mut input=action.input.clone();
            if self.default_action==Some(action.name) { input["required"].as_array_mut().expect("required array").retain(|field|field!="action"); input["properties"]["action"]["default"]=json!(action.name); }
            input
        }).collect::<Vec<_>>()})
        };
        let capabilities = self
            .fields
            .iter()
            .copied()
            .chain(
                self.actions
                    .iter()
                    .flat_map(|action| action.fields.iter().copied()),
            )
            .collect::<std::collections::BTreeSet<_>>();
        let mut result = json!({"name":self.name,"description":self.description,"inputSchema":input,"outputSchema":self.output,"annotations":{"readOnlyHint":self.read_only,"destructiveHint":!self.read_only,"idempotentHint":self.read_only,"openWorldHint":false},"x-wos-capabilities":capabilities});
        if !self.actions.is_empty() {
            result["x-wos-actions"]=json!(self.actions.iter().map(|action|json!({"name":action.name,"description":action.description,"fields":action.fields})).collect::<Vec<_>>());
            result["x-wos-default-action"] = json!(self.default_action);
        }
        result
    }
}
/// The exact tools/list payload, also used by command-line help and checked artifacts.
pub fn descriptors() -> Value {
    json!({"tools":TOOLS.iter().map(Tool::descriptor).collect::<Vec<_>>()})
}
/// One tool's executable declaration, suitable for command-line help.
pub fn help(name: &str) -> Option<Value> {
    find(name).map(Tool::descriptor)
}
/// Coverage is complete by construction over owning-schema root properties.
pub fn workflow_coverage() -> Value {
    let entries:Map<String,Value>=workflow_schema()["properties"].as_object().expect("Workflow properties").keys().map(|field|{
        let editors:Vec<String>=TOOLS.iter().flat_map(|tool|{
            let mut editors=vec![];
            if tool.fields.contains(&field.as_str()){editors.push(tool.name.to_string());}
            for action in &tool.actions{if action.fields.contains(&field.as_str()){editors.push(format!("{}.{}",tool.name,action.name));}}
            editors
        }).collect();
        (field.clone(),json!({"support":if editors.is_empty(){"import-export"}else{"incremental"},"editors":editors,"preservesNativeValues":true}))
    }).collect();
    json!({"workflowSchema":workflow_schema()["$id"],"fields":entries,"notes":["Imports and exports preserve every native value, including additional x-* root fields.","Complete Workflow readiness is separate from draft creation and payload-shape validation.","Replacing an entire named value intentionally replaces its descendants; other values remain unchanged.","Historical x-wos-governance and x-wos-ai values are retained but never written by native editors.","Reordering arrays and generic JSON patches are unsupported; use explicit edits or complete native import."]})
}

/// Read a declared default for direct owner-handler callers.
pub fn default_argument(tool: &str, argument: &str) -> Option<Value> {
    find(tool)?.input["properties"][argument]
        .get("default")
        .cloned()
}

/// Use the same declaration for direct handler callers that need boundary validation.
/// Returns normalized arguments; unknown names and unsupported inputs fail explicitly.
pub fn validate_arguments(tool: &str, args: &Value) -> Result<Value, ToolError> {
    find(tool)
        .ok_or_else(|| ToolError::InvalidArguments(format!("unknown tool: {tool}")))?
        .prepare_arguments(args.clone())
}
