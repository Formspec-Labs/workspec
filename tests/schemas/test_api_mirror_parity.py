"""ADR 0082 D-13 gate 6: every API `$def` carrying `x-wos.mirror` byte-aligns with its kernel source."""
import subprocess
import sys

from .test_wos_api_schema_discipline import ROOT


def test_api_mirrors_are_byte_aligned_with_their_kernel_sources():
    result = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "check-api-mirror-parity.py")],
        capture_output=True, text=True, check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
