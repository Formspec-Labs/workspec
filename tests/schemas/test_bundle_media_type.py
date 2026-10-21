"""Match public bundle metadata to maintained Trellis ZIP export bytes."""

import hashlib
import io
import json
from pathlib import Path
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parents[2]
VECTOR = ROOT.parent / "trellis/fixtures/vectors/export/001-two-event-chain"
# Maintained deterministic export vector, not a custom test ZIP writer.
VECTOR_SHA256 = "e0d6ef2bdc0756a5284e3cb66beb3d79c582b1f5e0231debbe44e41a162ba1fa"


def test_bundle_metadata_and_download_match_native_zip_export():
    manifest = tomllib.loads((VECTOR / "manifest.toml").read_text())
    assert manifest["expected"]["zip_sha256"] == VECTOR_SHA256
    archive = (VECTOR / manifest["expected"]["zip"]).read_bytes()
    assert hashlib.sha256(archive).hexdigest() == VECTOR_SHA256
    assert archive.startswith(b"PK\x03\x04")
    assert zipfile.is_zipfile(io.BytesIO(archive))
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        assert package.testzip() is None
        members = package.namelist()
        assert len(members) == len(set(members))
        manifest_members = [name for name in members if name.endswith("/000-manifest.cbor")]
        assert len(manifest_members) == 1
        assert package.read(manifest_members[0]) == (VECTOR / "000-manifest.cbor").read_bytes()
        assert any(name.endswith("/010-events.cbor") for name in members)

    openapi = json.loads((ROOT / "api/wos-public-api.openapi.json").read_text())
    content = openapi["paths"]["/api/v1/processes/{process_id}/completion-exports/{job_id}/download"]["get"]["responses"]["200"]["content"]
    assert set(content) == {"application/zip"}
    assert content["application/zip"]["schema"] == {"type": "string", "format": "binary"}
