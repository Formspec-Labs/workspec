// Rust guideline compliant 2026-02-21
//! Supplies exact synthetic responses through the maintained native service port.
use crate::reference_replay::FixtureInvocation;
use crate::{ConformanceError, StubService};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;
use wos_core::KernelDocument;
use wos_core::traits::ExternalService;
use wos_runtime::IntegrationProfileDocument;
use wos_runtime::runtime::reference_invocations::{
    reference_invocation_profile, validate_reference_invocation_response,
};

#[derive(Debug)]
struct SelectedInvocation {
    fixture: FixtureInvocation,
    binding: Value,
    identity: String,
}
/// Native fixture service with request validation before cache reuse.
#[derive(Debug)]
pub(crate) struct ReplayService {
    selected: Option<BTreeMap<String, SelectedInvocation>>,
    invocations: Mutex<BTreeMap<String, usize>>,
    legacy: StubService,
}
#[derive(Debug, thiserror::Error)]
#[error("selected invocation refused: {0}")]
pub(crate) struct InvocationError(String);
impl ReplayService {
    pub(crate) fn new(
        kernel: &KernelDocument,
        fixtures: Option<&[FixtureInvocation]>,
        legacy: StubService,
    ) -> Result<(Self, Option<IntegrationProfileDocument>), ConformanceError> {
        let profile = reference_invocation_profile(kernel)
            .map_err(|error| ConformanceError::Parse(error.to_string()))?;
        let selected = if let Some(fixtures) = fixtures {
            let mut selected = BTreeMap::new();
            let mut ids = std::collections::HashSet::new();
            for fixture in fixtures {
                let profile = profile.as_ref().ok_or_else(|| {
                    ConformanceError::Parse("fixture cannot grant invocation capability".into())
                })?;
                let binding = profile.bindings.get(&fixture.service_ref).ok_or_else(|| {
                    ConformanceError::Parse("unselected invocation fixture".into())
                })?;
                if fixture.id.is_empty()
                    || !ids.insert(&fixture.id)
                    || selected.contains_key(&fixture.service_ref)
                    || !(1..=1000).contains(&fixture.max_invocations)
                {
                    return Err(ConformanceError::Parse(
                        "invalid duplicate/limit invocation fixture".into(),
                    ));
                }
                validate_reference_invocation_response(
                    profile,
                    &fixture.service_ref,
                    &fixture.response,
                )
                .map_err(|error| ConformanceError::Parse(error.to_string()))?;
                let binding = serde_json::to_value(binding)
                    .map_err(|error| ConformanceError::Parse(error.to_string()))?;
                let original = json!({"fixture":fixture,"binding":binding});
                let bytes = integrity_canonical::canonical_json_bytes(&original)
                    .map_err(ConformanceError::Parse)?;
                let identity = integrity_canonical::compute_digest(
                    &bytes,
                    integrity_canonical::DigestAlgorithm::Sha256,
                );
                selected.insert(
                    fixture.service_ref.clone(),
                    SelectedInvocation {
                        fixture: fixture.clone(),
                        binding,
                        identity,
                    },
                );
            }
            if profile.as_ref().is_some_and(|profile| {
                profile
                    .bindings
                    .keys()
                    .any(|reference| !selected.contains_key(reference))
            }) {
                return Err(ConformanceError::Parse(
                    "missing selected invocation fixture".into(),
                ));
            }
            Some(selected)
        } else {
            if profile.is_some() {
                return Err(ConformanceError::Parse(
                    "selected invocation adapter required".into(),
                ));
            }
            None
        };
        Ok((
            Self {
                selected,
                invocations: Mutex::default(),
                legacy,
            },
            profile,
        ))
    }
    fn selected(
        &self,
        reference: &str,
        input: &Value,
    ) -> Result<&SelectedInvocation, InvocationError> {
        let selected = self
            .selected
            .as_ref()
            .and_then(|selected| selected.get(reference))
            .ok_or_else(|| InvocationError("missing selected provider".into()))?;
        if input != &selected.fixture.expected_input {
            return Err(InvocationError(
                "actual native request differs from selected expectedInput".into(),
            ));
        }
        Ok(selected)
    }
}
impl ExternalService for ReplayService {
    type Error = InvocationError;
    fn invocation_identity(
        &self,
        reference: &str,
        input: &Value,
        binding: Option<&Value>,
    ) -> Result<Option<String>, Self::Error> {
        if self.selected.is_none() {
            return Ok(None);
        }
        let selected = self.selected(reference, input)?;
        if binding != Some(&selected.binding) {
            return Err(InvocationError("actual selected binding changed".into()));
        }
        Ok(Some(selected.identity.clone()))
    }
    fn validate_cached_output(
        &self,
        reference: &str,
        input: &Value,
        output: &Value,
    ) -> Result<(), Self::Error> {
        if self.selected.is_none() {
            return Ok(());
        }
        let selected = self.selected(reference, input)?;
        if output != &selected.fixture.response {
            return Err(InvocationError(
                "persisted/provider response differs from selected fixture".into(),
            ));
        }
        Ok(())
    }
    fn invoke(
        &self,
        reference: &str,
        input: &Value,
        key: Option<&str>,
    ) -> Result<Value, Self::Error> {
        if self.selected.is_none() {
            return self
                .legacy
                .invoke(reference, input, key)
                .map_err(|error| InvocationError(error.to_string()));
        }
        let selected = self.selected(reference, input)?;
        let mut counts = self
            .invocations
            .lock()
            .map_err(|_| InvocationError("fixture result store unavailable".into()))?;
        let count = counts.entry(reference.into()).or_default();
        if *count >= selected.fixture.max_invocations {
            return Err(InvocationError("fixture invocation bound exceeded".into()));
        }
        *count += 1;
        Ok(selected.fixture.response.clone())
    }
}
