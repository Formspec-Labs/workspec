"""Application-case routes publish current producer admission and lookup absence."""
import json

import pytest

from .test_wos_api_schema_discipline import ROOT, load_openapi


@pytest.mark.parametrize("path,method", [
    ("/application-cases/start", "post"),
    ("/application-cases/by-draft/{draft_uuid}", "get"),
])
def test_application_case_routes_preserve_registered_transport_requirements(path, method):
    owning = json.loads((ROOT / "api/wos-public-api.registry.openapi.json").read_text())["paths"][path][method]
    public = load_openapi()["paths"][path][method]
    assert public["security"] == owning["security"] == [{"bearerAuth": []}]
    assert set(owning["responses"]) <= set(public["responses"])
    required = {(item["name"], item["in"], item["required"]) for item in owning["parameters"]}
    assert {(item["name"], item["in"], item["required"]) for item in public["parameters"]} == required
    if method == "post":
        assert public["requestBody"]["required"] is True
        assert "draftRef" in public["parameters"][0]["description"]
        assert "no accepted filing" in public["responses"]["200"]["description"]
    else:
        arms = public["responses"]["200"]["content"]["application/json"]["schema"]["oneOf"]
        assert any(arm.get("type") == "null" for arm in arms)
        assert any("$ref" in arm for arm in arms)
        assert "never creates" in public["responses"]["200"]["description"]
