#!/usr/bin/env python3
"""Emit base64-encoded COSE_Sign1 envelopes carrying a `method_uri`.

ADR 0109 consumer detached-signature shape: MAP_3 protected header with
`alg = -8` (EdDSA), 16-byte zero `kid`, and `method_uri` at COSE label `-65540`.
Signature payload is 64 zero bytes (partial-decode only; the WOS conformance
fixtures don't run the Formspec cryptographic helper, so a stub signature
suffices for binding pre-checks).

Output is paired with the Rust generator in `wos-formspec-binding`'s test
suite (`tests::cose_sign1_b64_with_method_uri`). The
`cose_b64_matches_python_generator` test pins both sides; both producers MUST
emit identical bytes.

Usage:
    python3 work-spec/scripts/gen-cose-sign1-method-uri.py <method_uri>...

If invoked without args, prints the two method_uri values used by the
SIG-027/028/029/030 conformance fixtures.
"""

import base64
import sys


def encode_uint(n: int) -> bytes:
    """CBOR major type 0 encoding (head + immediate bytes only — fixture
    values stay within the 32-bit range, so the 64-bit form is unused)."""
    if n < 24:
        return bytes([n])
    if n < 256:
        return bytes([0x18, n])
    if n < 65536:
        return bytes([0x19, n >> 8, n & 0xFF])
    if n < 2**32:
        return bytes([0x1A, (n >> 24) & 0xFF, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF])
    raise ValueError("uint too large")


def encode_negative_int(n: int) -> bytes:
    """CBOR major type 1: input is the unsigned magnitude `n` such that the
    CBOR value is `-1 - n`."""
    head = encode_uint(n)
    return bytes([0x20 | (head[0] & 0x1F)]) + head[1:]


def encode_bstr(b: bytes) -> bytes:
    """CBOR major type 2 (byte string)."""
    head = encode_uint(len(b))
    return bytes([0x40 | (head[0] & 0x1F)]) + head[1:] + b


def encode_tstr(s: str) -> bytes:
    """CBOR major type 3 (text string)."""
    b = s.encode("utf-8")
    head = encode_uint(len(b))
    return bytes([0x60 | (head[0] & 0x1F)]) + head[1:] + b


def encode_i128(n: int) -> bytes:
    if n >= 0:
        return encode_uint(n)
    return encode_negative_int(-1 - n)


def cose_label_bytes(label: int) -> bytes:
    if label >= 0:
        return encode_uint(label)
    return encode_negative_int(-1 - label)


COSE_LABEL_ALG = 1
COSE_LABEL_KID = 4
COSE_LABEL_METHOD_URI = -65540  # ADR 0109; owned by integrity-cose.
COSE_ALG_EDDSA = -8


def detached_signature_protected_header(alg: int, kid: bytes, method_uri: str) -> bytes:
    """Build MAP_3 protected header per ADR 0109 consumer envelope shape."""
    # 0xa3 = map(3); dCBOR canonical key order is alg(1) < kid(4) < method_uri(-65540)
    # because positive labels precede negative labels in dCBOR length-then-bytes ordering.
    out = bytes([0xA3])
    out += cose_label_bytes(COSE_LABEL_ALG) + encode_i128(alg)
    out += cose_label_bytes(COSE_LABEL_KID) + encode_bstr(kid)
    out += cose_label_bytes(COSE_LABEL_METHOD_URI) + encode_tstr(method_uri)
    return out


def encode_cose_sign1(protected_header: bytes, payload: bytes | None, signature: bytes) -> bytes:
    """Build tagged COSE_Sign1 envelope (RFC 9052)."""
    # 0xd2 = tag(18), 0x84 = array(4)
    out = bytes([0xD2, 0x84])
    out += encode_bstr(protected_header)
    out += bytes([0xA0])  # empty unprotected header map
    out += bytes([0xF6]) if payload is None else encode_bstr(payload)
    out += encode_bstr(signature)
    return out


def make_signature_value(method_uri: str) -> str:
    protected = detached_signature_protected_header(
        COSE_ALG_EDDSA, b"\x00" * 16, method_uri
    )
    envelope = encode_cose_sign1(protected, None, b"\x00" * 64)
    return base64.standard_b64encode(envelope).decode("ascii")


if __name__ == "__main__":
    if len(sys.argv) > 1:
        method_uris = sys.argv[1:]
    else:
        method_uris = [
            "urn:formspec:sig-method:ed25519-cose-sign1@1",
            "urn:formspec:sig-method:unknown@1",
        ]
    for m in method_uris:
        b64 = make_signature_value(m)
        print(f"{m!s:60s}\t{b64}")
