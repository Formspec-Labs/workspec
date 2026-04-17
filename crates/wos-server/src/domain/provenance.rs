use serde::{Deserialize, Serialize};

/// `ProvenanceRecord` in `WosBackend.ts:20`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRecordView {
    pub id: String,
    pub instance_id: String,
    pub timestamp: String,
    pub tier: String,
    pub actor: ActorRef,
    pub event: String,
    pub source_state: String,
    pub target_state: String,
    pub facts: FactsView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_narrative: Option<AiNarrativeView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterfactual: Option<CounterfactualView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_chain: Option<Vec<AuthorityLinkView>>,
    pub integrity: IntegrityView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorRef {
    pub id: String,
    #[serde(rename = "type")]
    pub actor_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactsView {
    pub inputs: serde_json::Value,
    pub outputs: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningView {
    pub rules_applied: Vec<String>,
    pub criteria_checked: Vec<CriteriaCheckView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CriteriaCheckView {
    pub label: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiNarrativeView {
    pub text: String,
    pub model: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterfactualView {
    pub positive: Vec<String>,
    pub negative: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityLinkView {
    pub actor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_instrument: Option<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityView {
    pub hash: String,
    pub previous_hash: String,
}
