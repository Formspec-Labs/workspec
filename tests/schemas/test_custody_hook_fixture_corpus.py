from __future__ import annotations

import hashlib
import json
import math
import struct
from pathlib import Path


FIXTURE_DIR = (
    Path(__file__).resolve().parents[2]
    / "fixtures"
    / "kernel"
    / "custody-hook"
    / "provenance-state-transition"
)


def _major_type(major: int, argument: int) -> bytes:
    if argument < 24:
        return bytes([(major << 5) | argument])
    if argument < 256:
        return bytes([(major << 5) | 24, argument])
    if argument < 65536:
        return bytes([(major << 5) | 25]) + struct.pack(">H", argument)
    if argument < 4294967296:
        return bytes([(major << 5) | 26]) + struct.pack(">I", argument)
    if argument < 18446744073709551616:
        return bytes([(major << 5) | 27]) + struct.pack(">Q", argument)
    raise ValueError("CBOR argument exceeds uint64 range")


def _encode_text(value: str) -> bytes:
    encoded = value.encode("utf-8")
    return _major_type(3, len(encoded)) + encoded


def _encode_json_to_dcbor(value: object, path: tuple[str, ...] = ()) -> bytes:
    if value is None:
        return b"\xf6"
    if value is False:
        return b"\xf4"
    if value is True:
        return b"\xf5"
    if isinstance(value, int):
        if value >= 0:
            return _major_type(0, value)
        return _major_type(1, -1 - value)
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError("non-finite float is not permitted")
        return b"\xfb" + struct.pack(">d", value)
    if isinstance(value, str):
        encoded = _encode_text(value)
        if path == ("timestamp",):
            return _major_type(6, 0) + encoded
        return encoded
    if isinstance(value, list):
        return _major_type(4, len(value)) + b"".join(
            _encode_json_to_dcbor(item, path) for item in value
        )
    if isinstance(value, dict):
        entries: list[tuple[bytes, bytes]] = []
        for key, item in value.items():
            key_bytes = _encode_text(key)
            value_bytes = _encode_json_to_dcbor(item, (*path, key))
            entries.append((key_bytes, key_bytes + value_bytes))
        entries.sort(key=lambda entry: entry[0])
        return _major_type(5, len(entries)) + b"".join(
            encoded_entry for _, encoded_entry in entries
        )
    raise TypeError(f"unsupported JSON value: {type(value)!r}")


def test_provenance_fixture_python_encoder_matches_committed_dcbor():
    record = json.loads((FIXTURE_DIR / "record.json").read_text())
    expected_bytes = (FIXTURE_DIR / "record.dcbor").read_bytes()
    expected_sha256 = (FIXTURE_DIR / "record.sha256").read_text().strip()

    encoded = _encode_json_to_dcbor(record)

    assert encoded == expected_bytes
    assert hashlib.sha256(encoded).hexdigest() == expected_sha256
