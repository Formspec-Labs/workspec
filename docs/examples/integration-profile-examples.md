<!-- relocated-from: profiles/integration.md §12 Examples per ADR 0076 D-8 + 2026-04-28 deletion. Non-normative example workflows demonstrating Integration Profile usage. The normative content (binding types, CloudEvents extension attributes, contract validation, correlation, idempotency) lives in `wos-spec/specs/kernel/spec.md` §9.2. -->

# Integration Profile Examples (non-normative)
## 1 Benefits Adjudication Integration Profile

This example demonstrates a complete Integration Profile for a benefits adjudication workflow.

```json
{
  "$wosIntegrationProfile": "1.0",
  "targetWorkflow": {
    "url": "https://agency.gov/workflows/benefits-adjudication",
    "compatibleVersions": ">=1.0.0 <2.0.0"
  },
  "bindings": {
    "eligibilityCheck": {
      "type": "arazzo-sequence",
      "description": "Multi-step eligibility verification via federal eligibility APIs",
      "arazzoRef": "urn:agency.gov:arazzo:eligibility-check:1.0.0",
      "responseContract": {
        "definitionRef": "urn:agency.gov:contracts:eligibility-response:1.0.0"
      },
      "inputMapping": {
        "applicantSSN": "caseFile.application.ssn",
        "householdSize": "caseFile.application.householdSize",
        "annualIncome": "caseFile.application.annualIncome"
      },
      "outputBinding": {
        "caseFile.eligibility.result": "$.steps.eligibility.output",
        "caseFile.eligibility.verifiedAt": "$.steps.verification.completedAt"
      },
      "idempotencyKeyExpression": "caseFile.application.id",
      "timeout": "PT5M",
      "retry": {
        "maxAttempts": 3,
        "backoff": "exponential",
        "initialInterval": "PT5S"
      }
    },
    "legacySystemCheck": {
      "type": "tool",
      "description": "Legacy mainframe eligibility cross-reference",
      "invocation": {
        "method": "command-line",
        "command": "/opt/legacy/eligibility-check",
        "arguments": [
          "--ssn", "{{ caseFile.application.ssn }}",
          "--household-size", "{{ caseFile.application.householdSize }}"
        ],
        "environment": {
          "image": "legacy-tools:2024.1"
        }
      },
      "responseContract": {
        "definitionRef": "urn:agency.gov:contracts:legacy-output:1.0.0"
      },
      "resourceRequirements": {
        "maxExecutionTime": "PT30S"
      }
    },
    "applicantNotification": {
      "type": "event-emit",
      "description": "Send notification event to applicant communication service",
      "eventType": "gov.agency.benefits.notification",
      "dataMapping": {
        "applicantId": "caseFile.application.applicantId",
        "noticeType": "caseFile.determination.noticeType",
        "determinationDate": "caseFile.determination.date"
      },
      "channel": "email"
    },
    "documentReceived": {
      "type": "event-consume",
      "description": "Receive uploaded supporting documents from document management system",
      "eventType": "gov.agency.documents.received",
      "correlation": [
        {
          "attribute": "subject",
          "caseStateMapping": "caseFile.application.applicationId"
        }
      ],
      "outputBinding": {
        "caseFile.documents.latest": "$.data.documentRef",
        "caseFile.documents.receivedAt": "$.time"
      }
    },
    "eligibilityPolicy": {
      "type": "policy-engine",
      "description": "OPA-based eligibility policy evaluation",
      "engineType": "opa",
      "endpoint": "https://policy.agency.gov/v1/data/benefits/eligibility",
      "contextMapping": {
        "input.applicant.income": "caseFile.application.annualIncome",
        "input.applicant.householdSize": "caseFile.application.householdSize",
        "input.applicant.state": "caseFile.application.stateOfResidence",
        "input.action": "'determineEligibility'",
        "input.actor": "event.actorId"
      },
      "decisionMapping": {
        "permitPath": "$.result.eligible",
        "reasonPath": "$.result.reason",
        "obligationsPath": "$.result.requiredDocuments"
      },
      "outputBinding": {
        "caseFile.policy.eligibilityDecision": "$.result"
      },
      "timeout": "PT5S"
    }
  }
}
```

---