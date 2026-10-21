"""Public instance read fields preserve the actual native identity model."""
from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, api_referencing_registry, load_schema, validator_for_api_def,
)


def test_hold_projection_has_native_fields_without_an_invented_identity():
    schema = load_schema(API_SCHEMA_DIR / "instance.schema.json")
    check = validator_for_api_def(schema, "HoldEntry", api_referencing_registry())
    value = {"holdType": "pending-applicant-response", "expectedDuration": "P7D",
             "engagedAt": "2026-10-20T12:00:00Z", "resumeTrigger": "applicant.response"}
    check.validate(value)
    assert not check.is_valid({**value, "id": "urn:wos:default_hold_01hw7rm71vfay8vvw14d2pf2db"})
