use axum::Json;
use axum::Router;
use axum::routing::get;

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/semantic/jsonld-context", get(jsonld_context))
}

static CONTEXT: &str = r#"{
  "@context": {
    "wos": "https://wos.dev/vocab#",
    "prov": "http://www.w3.org/ns/prov#",
    "xsd": "http://www.w3.org/2001/XMLSchema#",
    "instanceId": {"@id": "wos:instanceId", "@type": "@id"},
    "definitionUrl": {"@id": "wos:definitionUrl", "@type": "@id"},
    "definitionVersion": {"@id": "wos:definitionVersion", "@type": "xsd:string"},
    "caseState": {"@id": "wos:caseState", "@type": "@json"},
    "configuration": {"@id": "wos:configuration", "@container": "@set"},
    "impactLevel": {"@id": "wos:impactLevel", "@type": "xsd:string"},
    "recordKind": {"@id": "wos:recordKind", "@type": "xsd:string"},
    "actorId": {"@id": "wos:actorId", "@type": "@id"},
    "actorType": {"@id": "wos:actorType", "@type": "xsd:string"},
    "fromState": {"@id": "wos:fromState", "@type": "xsd:string"},
    "toState": {"@id": "wos:toState", "@type": "xsd:string"},
    "event": {"@id": "wos:event", "@type": "xsd:string"},
    "timestamp": {"@id": "wos:timestamp", "@type": "xsd:dateTime"},
    "ProvenanceRecord": "wos:ProvenanceRecord",
    "CaseInstance": "wos:CaseInstance",
    "KernelDocument": "wos:KernelDocument"
  }
}"#;

async fn jsonld_context() -> Json<serde_json::Value> {
    Json(serde_json::from_str(CONTEXT).unwrap_or_default())
}
