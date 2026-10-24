#!/usr/bin/env bash
# WS-094 Phase 4 (R-6.1): Restate Server (Docker) + wos-restate-worker + admin registration
# + ignored ingress integration test (`ingress_create_load_probe_smoke`).
#
# Requires: Docker, curl, bash. Optional but recommended: `nc` (netcat) for the worker
# TCP probe — macOS ships BSD `nc`; without `nc`, the script falls back to bash
# `/dev/tcp` (GNU/Linux bash only; macOS /bin/bash lacks /dev/tcp).
#
# Run from any cwd; resolves the wos-spec workspace root from this script's location.
# wos-server-runtime-restate has moved to workspec-server (chore 3.2); cargo commands
# below use --manifest-path to address the workspec-server workspace instead.
set -euo pipefail

WOS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLOWSPEC_SERVER_ROOT="$(cd "$WOS_ROOT/../workspec-server" && pwd)"
cd "$WOS_ROOT"

RESTATE_IMAGE="${WOS_RESTATE_SERVER_IMAGE:-docker.restate.dev/restatedev/restate:1.7.13}"
NAME="${WOS_RESTATE_DOCKER_NAME:-wos-restate-ci-$$}"

# Request identity (ADR 0164 section 1.3): Restate signs its requests with a throwaway Ed25519 key and
# the worker verifies them. The private key stays in this temp dir (never printed).
KEY_DIR="$(mktemp -d)"
"$(dirname "$0")/restate_identity_key.sh" "$KEY_DIR"
chmod 0644 "$KEY_DIR/request-identity.pem" # throwaway key; the container user must read the bind mount
WOS_RESTATE_REQUEST_IDENTITY_KEYS="$(cat "$KEY_DIR/request-identity.pub")"
export WOS_RESTATE_REQUEST_IDENTITY_KEYS

# WOS_RESTATE_ALWAYS_REPLAY=1: inactivity timeout 0 makes Restate suspend and replay at every await.
REPLAY_ENV=()
if [[ "${WOS_RESTATE_ALWAYS_REPLAY:-}" == "1" ]]; then
  REPLAY_ENV=(-e RESTATE_WORKER__INVOKER__INACTIVITY_TIMEOUT=0m)
fi

docker rm -f "$NAME" >/dev/null 2>&1 || true
# Published on 127.0.0.1 only. This is the loopback-only exception to the closed Restate plane: the
# worker and the ignored ingress tests run on the host, so they need ingress (8080) and admin (9070);
# the node port (5122) is not published. Restate reaches the host worker via host.docker.internal.
docker run -d --name "$NAME" \
  --add-host=host.docker.internal:host-gateway \
  -p 127.0.0.1:8080:8080 -p 127.0.0.1:9070:9070 \
  -v "$KEY_DIR/request-identity.pem:/run/request-identity.pem:ro" \
  -e RESTATE_WORKER__INVOKER__REQUEST_IDENTITY_PRIVATE_KEY_PEM_FILE=/run/request-identity.pem \
  ${REPLAY_ENV[@]+"${REPLAY_ENV[@]}"} \
  "$RESTATE_IMAGE" \
  --node-name=wos-ci-node

cleanup() {
  if [[ -n "${WORKER_PID:-}" ]]; then
    kill "$WORKER_PID" 2>/dev/null || true
    wait "$WORKER_PID" 2>/dev/null || true
  fi
  docker stop "$NAME" >/dev/null 2>&1 || true
  docker rm "$NAME" >/dev/null 2>&1 || true
  rm -rf "$KEY_DIR"
}
trap cleanup EXIT

echo "Waiting for Restate admin (9070)..."
for _ in $(seq 1 90); do
  if curl -sf "http://127.0.0.1:9070/deployments" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
if ! curl -sf "http://127.0.0.1:9070/deployments" >/dev/null 2>&1; then
  echo "Restate admin did not become reachable in time" >&2
  exit 1
fi

echo "Building wos-restate-worker..."
cargo build --manifest-path "$FLOWSPEC_SERVER_ROOT/Cargo.toml" -p wos-server-runtime-restate --bin wos-restate-worker

echo "Starting worker on :9080..."
"$FLOWSPEC_SERVER_ROOT/target/debug/wos-restate-worker" &
WORKER_PID=$!

# Worker serves HTTP/2 prior-knowledge; probe TCP listen (nc when available, else bash /dev/tcp on Linux).
for _ in $(seq 1 30); do
  if command -v nc >/dev/null 2>&1 && nc -z 127.0.0.1 9080 2>/dev/null; then
    break
  fi
  if bash -c "exec 3<>/dev/tcp/127.0.0.1/9080" 2>/dev/null; then
    break
  fi
  sleep 1
done

echo "Registering deployment with Restate..."
curl -sSf "http://127.0.0.1:9070/deployments" \
  -H "content-type: application/json" \
  -d '{"uri":"http://host.docker.internal:9080","force":true}'

export WOS_RESTATE_IT_URL=http://127.0.0.1:8080
echo "Running ingress smoke tests (ignored tests, require Restate)..."
cargo test --manifest-path "$FLOWSPEC_SERVER_ROOT/Cargo.toml" -p wos-server-runtime-restate ingress_create_load_probe_smoke -- --ignored --nocapture
cargo test --manifest-path "$FLOWSPEC_SERVER_ROOT/Cargo.toml" -p wos-server-runtime-restate ingress_drain_lifecycle_smoke -- --ignored --nocapture
cargo test --manifest-path "$FLOWSPEC_SERVER_ROOT/Cargo.toml" -p wos-server-runtime-restate ingress_duplicate_create_is_terminal -- --ignored --nocapture
cargo test --manifest-path "$FLOWSPEC_SERVER_ROOT/Cargo.toml" -p wos-server-runtime-restate ingress_malformed_event_is_terminal -- --ignored --nocapture
cargo test --manifest-path "$FLOWSPEC_SERVER_ROOT/Cargo.toml" -p wos-server-runtime-restate ingress_load_nonexistent_is_terminal -- --ignored --nocapture

echo "Restate ingress smoke OK (B.0 + B.1 + D.1 terminal)."
