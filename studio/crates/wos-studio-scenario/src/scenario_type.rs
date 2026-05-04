// Rust guideline compliant 2026-05-02

//! The 16 canonical scenario types per `scenario-authoring.md` §"Scenario types".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioType {
    HappyPath,
    IncompleteApplication,
    DeadlineMissed,
    AdverseDetermination,
    NoticeGenerated,
    AppealFiled,
    ExceptionApplies,
    SupportingDocumentMissing,
    ManualOverride,
    SystemFailureFallback,
    AgentFallback,
    PolicyChange,
    EquityProbe,
    AccessibilityCheck,
    JurisdictionalVariation,
    RuntimeObservationReplay,
}

impl ScenarioType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HappyPath => "happy-path",
            Self::IncompleteApplication => "incomplete-application",
            Self::DeadlineMissed => "deadline-missed",
            Self::AdverseDetermination => "adverse-determination",
            Self::NoticeGenerated => "notice-generated",
            Self::AppealFiled => "appeal-filed",
            Self::ExceptionApplies => "exception-applies",
            Self::SupportingDocumentMissing => "supporting-document-missing",
            Self::ManualOverride => "manual-override",
            Self::SystemFailureFallback => "system-failure-fallback",
            Self::AgentFallback => "agent-fallback",
            Self::PolicyChange => "policy-change",
            Self::EquityProbe => "equity-probe",
            Self::AccessibilityCheck => "accessibility-check",
            Self::JurisdictionalVariation => "jurisdictional-variation",
            Self::RuntimeObservationReplay => "runtime-observation-replay",
        }
    }
}

pub fn parse_scenario_type(s: &str) -> Option<ScenarioType> {
    match s {
        "happy-path" => Some(ScenarioType::HappyPath),
        "incomplete-application" => Some(ScenarioType::IncompleteApplication),
        "deadline-missed" => Some(ScenarioType::DeadlineMissed),
        "adverse-determination" => Some(ScenarioType::AdverseDetermination),
        "notice-generated" => Some(ScenarioType::NoticeGenerated),
        "appeal-filed" => Some(ScenarioType::AppealFiled),
        "exception-applies" => Some(ScenarioType::ExceptionApplies),
        "supporting-document-missing" => Some(ScenarioType::SupportingDocumentMissing),
        "manual-override" => Some(ScenarioType::ManualOverride),
        "system-failure-fallback" => Some(ScenarioType::SystemFailureFallback),
        "agent-fallback" => Some(ScenarioType::AgentFallback),
        "policy-change" => Some(ScenarioType::PolicyChange),
        "equity-probe" => Some(ScenarioType::EquityProbe),
        "accessibility-check" => Some(ScenarioType::AccessibilityCheck),
        "jurisdictional-variation" => Some(ScenarioType::JurisdictionalVariation),
        "runtime-observation-replay" => Some(ScenarioType::RuntimeObservationReplay),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_sixteen_types() {
        let all = [
            "happy-path",
            "incomplete-application",
            "deadline-missed",
            "adverse-determination",
            "notice-generated",
            "appeal-filed",
            "exception-applies",
            "supporting-document-missing",
            "manual-override",
            "system-failure-fallback",
            "agent-fallback",
            "policy-change",
            "equity-probe",
            "accessibility-check",
            "jurisdictional-variation",
            "runtime-observation-replay",
        ];
        assert_eq!(all.len(), 16);
        for s in all {
            let parsed = parse_scenario_type(s).expect(s);
            assert_eq!(parsed.as_str(), s);
            let json = serde_json::to_string(&parsed).unwrap();
            assert_eq!(json, format!("\"{s}\""));
        }
    }
}
