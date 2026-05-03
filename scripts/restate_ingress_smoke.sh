#!/usr/bin/env bash
# WS-094 Phase 4 (R-6.1): Restate Server (Docker) + wos-restate-worker + admin registration
# + ignored ingress integration test (`ingress_create_load_probe_smoke`).
#
# Requires: Docker, curl, bash. Optional but recommended: `nc` (netcat) for the worker
# TCP probe — macOS ships BSD `nc`; without `nc`, the script falls back to bash
# `/dev/tcp` (GNU/Linux bash only; macOS /bin/bash lacks /dev/tcp).
#
# Run from any cwd; resolves the wos-spec workspace root from this script's location.
set -euo pipefail

WOS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WOS_ROOT"

RESTATE_IMAGE="${WOS_RESTATE_SERVER_IMAGE:-docker.restate.dev/restatedev/restate:1.6.2}"
NAME="${WOS_RESTATE_DOCKER_NAME:-wos-restate-ci-$$}"

docker rm -f "$NAME" >/dev/null 2>&1 || true
docker run -d --name "$NAME" \
  --add-host=host.docker.internal:host-gateway \
  -p 8080:8080 -p 9070:9070 -p 5122:5122 \
  "$RESTATE_IMAGE" \
  --node-name=wos-ci-node

cleanup() {
  if [[ -n "${WORKER_PID:-}" ]]; then
    kill "$WORKER_PID" 2>/dev/null || true
    wait "$WORKER_PID" 2>/dev/null || true
  fi
  docker stop "$NAME" >/dev/null 2>&1 || true
  docker rm "$NAME" >/dev/null 2>&1 || true
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
cargo build -p wos-server-runtime-restate --bin wos-restate-worker

echo "Starting worker on :9080..."
./target/debug/wos-restate-worker &
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
cargo test -p wos-server-runtime-restate ingress_create_load_probe_smoke -- --ignored --nocapture
cargo test -p wos-server-runtime-restate ingress_drain_lifecycle_smoke -- --ignored --nocapture
cargo test -p wos-server-runtime-restate ingress_duplicate_create_is_terminal -- --ignored --nocapture
cargo test -p wos-server-runtime-restate ingress_malformed_event_is_terminal -- --ignored --nocapture
cargo test -p wos-server-runtime-restate ingress_load_nonexistent_is_terminal -- --ignored --nocapture

echo "Restate ingress smoke OK (B.0 + B.1 + D.1 terminal)."
