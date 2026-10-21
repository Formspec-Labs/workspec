#!/bin/sh
# Writes a Restate request-identity key pair (ADR 0164 section 1.3) into <dir>:
#   request-identity.pem   PKCS#8 Ed25519 private key Restate signs its requests with (never printed)
#   request-identity.pub   the SDK public key, "publickeyv1_" + base58 of the raw 32-byte public key,
#                          which every WOS Restate endpoint verifies against
# The one place this encoding lives; the D1 dev profile and the ingress smoke test both call it.
set -eu
dir=${1:?usage: restate_identity_key.sh <dir>}
mkdir -p "$dir"
openssl genpkey -algorithm ed25519 -out "$dir/request-identity.pem"
openssl pkey -in "$dir/request-identity.pem" -pubout -outform DER | tail -c 32 | python3 -I -c '
import sys
alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
raw = sys.stdin.buffer.read()
n = int.from_bytes(raw, "big")
out = ""
while n:
    n, r = divmod(n, 58)
    out = alphabet[r] + out
sys.stdout.write("publickeyv1_" + "1" * (len(raw) - len(raw.lstrip(b"\0"))) + out)' > "$dir/request-identity.pub"
