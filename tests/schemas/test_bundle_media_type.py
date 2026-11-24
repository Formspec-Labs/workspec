"""Check the public completion-export status and download media types."""

from .test_wos_api_schema_discipline import load_openapi


def test_bundle_status_returns_job_metadata_and_download_returns_binary_zip():
    paths = load_openapi()["paths"]
    job_path = "/api/v1/processes/{process_id}/completion-exports/{job_id}"

    status = paths[job_path]["get"]["responses"]["200"]["content"]
    assert set(status) == {"application/json"}
    assert status["application/json"]["schema"]["$ref"] == "#/components/schemas/CompletionExportJob"

    content = paths[job_path + "/download"]["get"]["responses"]["200"]["content"]
    assert set(content) == {"application/zip"}
    schema = content["application/zip"]["schema"]
    assert schema["type"] == "string"
    assert schema["format"] == "binary"
