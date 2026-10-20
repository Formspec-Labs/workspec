// Rust guideline compliant 2026-02-21
//! Immutable native fixture capabilities, separate from authored event data.
use crate::ConformanceError;
use crate::studio_api::{PreparedReplay, ReplayRequest, ReplayResult};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ring::signature::KeyPair;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use wos_core::instance::HumanActorBinding;
use wos_core::model::notification_template::NotificationTemplateDocument;
use wos_runtime::SelectedDocumentSigningKey;
use wos_runtime::runtime::governance::EvidenceSelection;

struct OfflineRetriever;
impl jsonschema::Retrieve for OfflineRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let text = match uri.as_str() {
            "https://wos-spec.org/schemas/delivery/1.0" => {
                include_str!("../../../schemas/sidecars/wos-delivery.schema.json")
            }
            "https://wos-spec.org/schemas/wos-reference-replay.schema.json" => {
                include_str!("../../../schemas/wos-reference-replay.schema.json")
            }
            "https://wos-spec.org/schemas/wos-reference-due-process.schema.json" => {
                include_str!("../../../schemas/wos-reference-due-process.schema.json")
            }
            "https://wos-spec.org/schemas/wos-workflow.schema.json"
            | "https://wos-spec.org/schemas/workflow/1.0" => {
                include_str!("../../../schemas/wos-workflow.schema.json")
            }
            _ => {
                return Err(format!("reference schema resource unavailable locally: {uri}").into());
            }
        };
        Ok(serde_json::from_str(text)?)
    }
}

pub(crate) fn validate_reference_fragment(
    name: &str,
    value: &Value,
) -> Result<(), ConformanceError> {
    static VALIDATORS: std::sync::LazyLock<BTreeMap<&'static str, jsonschema::Validator>> =
        std::sync::LazyLock::new(|| {
            ["GovernanceRecipe", "SignatureRecipe", "NotificationTemplateDocument", "Delivery"].into_iter().map(|name| {
            let validator = jsonschema::options().with_retriever(OfflineRetriever)
                .build(&serde_json::json!({"$ref":if name=="Delivery" {"https://wos-spec.org/schemas/delivery/1.0".to_string()}else{format!("https://wos-spec.org/schemas/wos-reference-replay.schema.json#/$defs/{name}")}}))
                .expect("bundled native recipe schema compiles");
            (name, validator)
        }).collect()
        });
    let validator = VALIDATORS
        .get(name)
        .ok_or_else(|| ConformanceError::Parse("unknown native recipe schema".into()))?;
    if let Some(error) = validator.iter_errors(value).next() {
        return Err(ConformanceError::Parse(format!("invalid {name}: {error}")));
    }
    Ok(())
}

fn adapter_validator() -> &'static jsonschema::Validator {
    static VALIDATOR: std::sync::LazyLock<jsonschema::Validator> = std::sync::LazyLock::new(|| {
        let mut registry = jsonschema::Registry::new().retriever(OfflineRetriever);
        for (id, bytes) in [
            (
                "https://wos-spec.org/schemas/wos-reference-replay.schema.json",
                include_str!("../../../schemas/wos-reference-replay.schema.json"),
            ),
            (
                "https://wos-spec.org/schemas/wos-reference-due-process.schema.json",
                include_str!("../../../schemas/wos-reference-due-process.schema.json"),
            ),
            (
                "https://wos-spec.org/schemas/wos-workflow.schema.json",
                include_str!("../../../schemas/wos-workflow.schema.json"),
            ),
            (
                "https://wos-spec.org/schemas/workflow/1.0",
                include_str!("../../../schemas/wos-workflow.schema.json"),
            ),
        ] {
            registry = registry
                .add(
                    id,
                    serde_json::from_str::<Value>(bytes).expect("bundled schema JSON"),
                )
                .expect("bundled schema resource");
        }
        let registry = registry
            .prepare()
            .expect("bundled reference schema references resolve");
        jsonschema::options().with_registry(&registry)
            .build(&serde_json::json!({"$ref":"https://wos-spec.org/schemas/wos-reference-replay.schema.json"}))
            .expect("bundled reference adapter schema compiles")
    });
    &VALIDATOR
}

fn validate_adapters(adapters: &ReferenceReplayAdapters) -> Result<(), ConformanceError> {
    let invalid =
        |message: String| ConformanceError::Parse(format!("reference adapters: {message}"));
    let value = serde_json::to_value(adapters).map_err(|error| invalid(error.to_string()))?;
    if let Some(error) = adapter_validator().iter_errors(&value).next() {
        return Err(invalid(error.to_string()));
    }
    for values in [
        adapters
            .document_signing_keys
            .iter()
            .map(|key| key.id.as_str())
            .collect::<Vec<_>>(),
        adapters
            .trusted_receipt_signers
            .iter()
            .map(|key| key.id.as_str())
            .collect(),
        adapters
            .fixture_signing_capabilities
            .iter()
            .map(|key| key.id.as_str())
            .collect(),
    ] {
        if values
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != values.len()
        {
            return Err(invalid("duplicate selected key/capability identity".into()));
        }
    }
    let decode = |value: &str| -> Result<Vec<u8>, ConformanceError> {
        let bytes = STANDARD
            .decode(value)
            .map_err(|error| invalid(error.to_string()))?;
        if STANDARD.encode(&bytes) != value {
            return Err(invalid("noncanonical key bytes".into()));
        }
        Ok(bytes)
    };
    for capability in &adapters.fixture_signing_capabilities {
        let key = adapters
            .document_signing_keys
            .iter()
            .find(|key| key.id == capability.document_key_ref)
            .ok_or_else(|| invalid("missing selected document key".into()))?;
        let receipt = adapters
            .trusted_receipt_signers
            .iter()
            .find(|key| key.id == capability.receipt_signer_ref)
            .ok_or_else(|| invalid("missing selected receipt key".into()))?;
        let document_pair =
            ring::signature::Ed25519KeyPair::from_pkcs8(&decode(&capability.pkcs8_base64)?)
                .map_err(|_| invalid("invalid document private key".into()))?;
        let receipt_pair =
            ring::signature::Ed25519KeyPair::from_pkcs8(&decode(&capability.receipt_pkcs8_base64)?)
                .map_err(|_| invalid("invalid receipt private key".into()))?;
        if document_pair.public_key().as_ref() != decode(&key.public_key_base64)?
            || receipt_pair.public_key().as_ref() != decode(&receipt.public_key_base64)?
        {
            return Err(invalid(
                "selected fixture private/public key mismatch".into(),
            ));
        }
    }
    let schedule = &adapters.notice_transport.failure_schedule;
    if schedule
        .iter()
        .map(|failure| failure.attempt)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != schedule.len()
    {
        return Err(invalid("duplicate selected transport attempt".into()));
    }
    Ok(())
}

/// Explicit local software-fixture authority boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReferenceScope {
    SoftwareFixture,
}
/// Receipt trust is independent of document-signing trust.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureReceiptSigner {
    /// Selected signer identity.
    pub id: String,
    /// Supported suite, exactly ed25519.
    pub suite: String,
    /// Canonical base64 protected kid.
    pub kid_base64: String,
    /// Canonical base64 public key.
    pub public_key_base64: String,
    /// Selected receipt method.
    pub method_uri: String,
}
/// Numbered explicit no-effect fixture failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureTransportFailure {
    /// Actual one-based invocation ordinal.
    pub attempt: usize,
    /// Exactly noEffectFailure.
    pub outcome: String,
}
/// Selected recoverable local notice transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureNoticeTransport {
    /// Selected adapter identity.
    pub id: String,
    /// Explicit local scope.
    pub scope: ReferenceScope,
    /// Exactly sameInstantQualifiedReceipt.
    pub mode: String,
    /// Selected identity of this isolated retained-results store.
    pub retained_results_identity: String,
    /// Explicit failures; subsequent actual attempts use the selected mode.
    pub failure_schedule: Vec<FixtureTransportFailure>,
}
/// Synthetic fixture key material; never selected by a generic event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureSigningCapability {
    /// Selected signing recipe capability.
    pub id: String,
    /// Explicit software-fixture scope.
    pub scope: ReferenceScope,
    /// Selected immutable source document key.
    pub document_key_ref: String,
    /// Canonical base64 fixture private key.
    pub pkcs8_base64: String,
    /// Separate selected receipt key.
    pub receipt_signer_ref: String,
    /// Canonical base64 fixture receipt private key.
    pub receipt_pkcs8_base64: String,
}
/// Explicit synthetic external request/result fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureInvocation {
    /// Unique fixture identity.
    pub id: String,
    /// Selected native invocation declaration.
    pub service_ref: String,
    /// Exact actual native request required before invocation or cache reuse.
    pub expected_input: Value,
    /// Explicit synthetic provider output.
    pub response: Value,
    /// Bounded actual invocation count.
    pub max_invocations: usize,
}
/// Immutable selected native capabilities; JSON event data cannot construct them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTimeAdvance {
    /// Original event index before which the native clock advances.
    pub before_event_index: usize,
    /// Canonical whole-day ISO duration, such as P10D.
    pub duration: String,
}
/// Immutable selected native capabilities; JSON event data cannot construct them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceReplayAdapters {
    /// Explicit native clock advances, retained separately from original event bytes.
    #[serde(default)]
    pub time_advances: Vec<NativeTimeAdvance>,
    /// Explicit local scope.
    pub scope: ReferenceScope,
    /// Actual immutable local-role to canonical-person assignments.
    pub human_actor_bindings: Vec<HumanActorBinding>,
    /// Selected signed receipt trust roots.
    pub trusted_receipt_signers: Vec<FixtureReceiptSigner>,
    /// Selected local notice transport.
    pub notice_transport: FixtureNoticeTransport,
    /// Retained source, review and actual hearing/override evidence.
    pub retained_evidence: Vec<EvidenceSelection>,
    /// Selected native template DTOs, distinct from complete Delivery documents.
    #[serde(deserialize_with = "deserialize_templates")]
    pub templates: Vec<NotificationTemplateDocument>,
    /// Selected source document keys.
    pub document_signing_keys: Vec<SelectedDocumentSigningKey>,
    /// Explicit synthetic signing capabilities.
    pub fixture_signing_capabilities: Vec<FixtureSigningCapability>,
    /// Explicit synthetic service and policy fixtures.
    pub invocations: Vec<FixtureInvocation>,
}

fn deserialize_templates<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<NotificationTemplateDocument>, D::Error> {
    let values = Vec::<Value>::deserialize(deserializer)?;
    values
        .into_iter()
        .map(|value| {
            validate_reference_fragment("NotificationTemplateDocument", &value)
                .map_err(serde::de::Error::custom)?;
            serde_json::from_value(value).map_err(serde::de::Error::custom)
        })
        .collect()
}

pub(crate) fn lower_time_advances(
    fixture: &mut Value,
    declarations: &[NativeTimeAdvance],
) -> Result<(), ConformanceError> {
    let events = fixture
        .get_mut("event_sequence")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| ConformanceError::Parse("time advances require an event sequence".into()))?;
    let mut indices = std::collections::BTreeSet::new();
    for advance in declarations {
        let days = advance
            .duration
            .strip_prefix('P')
            .and_then(|value| value.strip_suffix('D'));
        let canonical = days
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|value| {
                (1..=36500).contains(&value) && advance.duration == format!("P{value}D")
            });
        if !canonical || !indices.insert(advance.before_event_index) {
            return Err(ConformanceError::Parse(
                "invalid or duplicate native time advance".into(),
            ));
        }
        let event = events
            .get_mut(advance.before_event_index)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                ConformanceError::Parse("native time advance index out of range".into())
            })?;
        if event.contains_key("delay") {
            return Err(ConformanceError::Parse(
                "native time advance conflicts with event delay".into(),
            ));
        }
        wos_core::eval::parse_iso_duration_to_ms(&advance.duration)
            .map_err(|error| ConformanceError::Parse(format!("invalid time advance: {error}")))?;
        event.insert("delay".into(), Value::String(advance.duration.clone()));
    }
    Ok(())
}
/// Frozen native reference inputs, retaining the exact original request.
#[derive(Debug, Clone)]
pub struct PreparedReferenceReplay {
    pub(crate) prepared: PreparedReplay,
    request: ReplayRequest,
    adapters: ReferenceReplayAdapters,
    evidence: BTreeMap<(String, String), Vec<u8>>,
}
impl PreparedReferenceReplay {
    /// Returns the complete frozen native input identity.
    pub fn identity(&self) -> &str {
        self.prepared.identity()
    }
    /// Returns the current compiled native source identity and configuration.
    pub fn runtime_identity(&self) -> &Value {
        self.prepared.runtime_identity()
    }
    /// Returns the unchanged complete original request.
    pub fn request(&self) -> &ReplayRequest {
        &self.request
    }
    /// Returns the immutable original adapter choices.
    pub fn adapters(&self) -> &ReferenceReplayAdapters {
        &self.adapters
    }
    /// Returns validated original evidence bytes by exact selected identity and role.
    pub fn retained_evidence(&self, id: &str, role: &str) -> Option<&[u8]> {
        self.evidence
            .get(&(id.into(), role.into()))
            .map(Vec::as_slice)
    }
    /// Returns original values for exact paired input comparison.
    pub fn original_inputs(&self) -> Value {
        serde_json::json!({"request":self.request,"adapters":self.adapters,
            "fixture":serde_json::from_str::<Value>(&self.request.fixture_json).expect("prepared fixture parsed")})
    }
}
/// Prepares actual native capabilities before workflow activation.
///
/// # Errors
/// Refuses unresolved inputs, unsupported selected capabilities or invalid recipes.
pub fn prepare_reference_replay(
    request: ReplayRequest,
    adapters: ReferenceReplayAdapters,
) -> Result<PreparedReferenceReplay, ConformanceError> {
    validate_adapters(&adapters)?;
    let mut evidence = BTreeMap::new();
    let mut ids = std::collections::BTreeSet::new();
    for selected in &adapters.retained_evidence {
        let bytes = STANDARD
            .decode(&selected.bytes_base64)
            .map_err(|error| ConformanceError::Parse(error.to_string()))?;
        let role_value = serde_json::to_value(&selected.role)
            .map_err(|error| ConformanceError::Parse(error.to_string()))?;
        let role = role_value
            .as_str()
            .ok_or_else(|| ConformanceError::Parse("invalid evidence role".into()))?
            .to_string();
        if selected.id.is_empty()
            || !ids.insert(selected.id.clone())
            || bytes.len() > 10_485_760
            || STANDARD.encode(&bytes) != selected.bytes_base64
            || format!(
                "sha256:{}",
                integrity_canonical::compute_digest(
                    &bytes,
                    integrity_canonical::DigestAlgorithm::Sha256,
                )
            ) != selected.sha256
        {
            return Err(ConformanceError::Parse(
                "invalid selected retained evidence".into(),
            ));
        }
        evidence.insert((selected.id.clone(), role), bytes);
    }
    let prepared =
        crate::studio_api::prepare_replay_with_reference(request.clone(), Some(&adapters))?;
    Ok(PreparedReferenceReplay {
        prepared,
        request,
        adapters,
        evidence,
    })
}
/// Executes the exact frozen reference inputs through maintained native owners.
///
/// # Errors
/// Returns native preparation or observation errors with original execution retained.
pub fn replay_reference(
    prepared: &PreparedReferenceReplay,
) -> Result<ReplayResult, ConformanceError> {
    crate::studio_api::replay(&prepared.prepared)
}
