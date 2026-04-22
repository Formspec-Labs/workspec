"""Shared helpers for WOS schema regression tests.

Provides:
- WOS_SPEC_ROOT / SCHEMAS_ROOT constants for repo-relative path building.
- MARKER_TO_SCHEMA — the authoritative mapping from each `$wos*` document
  marker to its declaring schema file. When a new schema is added to
  `schemas/`, add its `$wos*` marker here and every fixture + spec code
  block carrying that marker becomes part of the regression suite
  automatically.
- A session-scoped `validators` fixture that compiles every schema once.
- `classify(doc)` — returns the `$wos*` marker key present in a document,
  or None if the document is unmarked (e.g. negative-fixture catalogs or
  non-WOS auxiliary data).
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
SCHEMAS_ROOT = WOS_SPEC_ROOT / "schemas"

# Marker → schema path. Markers MUST match the `$wos*` property each schema
# declares as required at its root. Verified against every schema in
# `schemas/` on 2026-04-17.
MARKER_TO_SCHEMA: dict[str, str] = {
    "$wosKernel": "kernel/wos-kernel.schema.json",
    "$wosCorrespondenceMetadata": "kernel/wos-correspondence-metadata.schema.json",
    "$wosLifecycleDetail": "companions/wos-lifecycle-detail.schema.json",
    "$wosWorkflowGovernance": "governance/wos-workflow-governance.schema.json",
    "$wosAssertionLibrary": "governance/wos-assertion-gate.schema.json",
    "$wosDueProcess": "governance/wos-due-process.schema.json",
    "$wosPolicyParameters": "governance/wos-policy-parameters.schema.json",
    "$wosAIIntegration": "ai/wos-ai-integration.schema.json",
    "$wosAgentConfig": "ai/wos-agent-config.schema.json",
    "$wosDriftMonitor": "ai/wos-drift-monitor.schema.json",
    "$wosAdvancedGovernance": "advanced/wos-advanced.schema.json",
    "$wosEquityConfig": "advanced/wos-equity.schema.json",
    "$wosVerificationReport": "advanced/wos-verification-report.schema.json",
    "$wosIntegrationProfile": "profiles/wos-integration-profile.schema.json",
    "$wosSemanticProfile": "profiles/wos-semantic-profile.schema.json",
    "$wosSignatureProfile": "profiles/wos-signature-profile.schema.json",
    "$wosBusinessCalendar": "sidecars/wos-business-calendar.schema.json",
    "$wosNotificationTemplate": "sidecars/wos-notification-template.schema.json",
    "$wosExtensionRegistry": "registry/wos-extension-registry.schema.json",
}


@pytest.fixture(scope="session")
def validators() -> dict[str, Draft202012Validator]:
    """Load and compile every classified schema once per test session."""
    compiled: dict[str, Draft202012Validator] = {}
    for marker, rel in MARKER_TO_SCHEMA.items():
        schema = json.loads((SCHEMAS_ROOT / rel).read_text())
        compiled[marker] = Draft202012Validator(schema)
    return compiled


def classify(doc: object) -> str | None:
    """Return the first `$wos*` marker key in a document, or None.

    Documents without a marker are auxiliary artifacts (negative-fixture
    catalogs, scenario transcripts) that the regression suite skips.
    """
    if not isinstance(doc, dict):
        return None
    for key in doc:
        if isinstance(key, str) and key.startswith("$wos"):
            return key
    return None
