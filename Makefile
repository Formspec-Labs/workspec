# WOS (Workflow Orchestration Standard) Makefile

# Variables
CARGO = cargo
PYTEST = python3 -m pytest

# Targets
.PHONY: all build test test-core lint clean help \
	rust-build rust-test rust-check \
	python-test \
	check-openapi-parity \
	postgres-up postgres-down \
	restate-ingress-smoke

# Default target: build and test everything (Rust + Python schema regression)
all: build test

help:
	@echo "WOS (Workflow Orchestration Standard) Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Primary Targets:"
	@echo "  all                  Build and test everything"
	@echo "  build                Build the Rust workspace"
	@echo "  test                 Run all tests (Rust, Python schema regression)"
	@echo "  test-core            Same as test (alias)"
	@echo "  lint                 Run all linters and checks (rust-check + check-openapi-parity)"
	@echo "  check-openapi-parity Wave 7 Finding A1 — assert registry OpenAPI response codes are mirrored in broader OpenAPI"
	@echo "  clean                Remove build artifacts"
	@echo ""
	@echo "Parent Rust Targets (wos-spec workspace):"
	@echo "  rust-build           Build all parent Rust crates"
	@echo "  rust-test            Run all parent Rust tests"
	@echo "  rust-check           Run cargo check on the parent workspace"
	@echo ""
	@echo "Parent Python Targets:"
	@echo "  python-test          Run parent Python schema-conformance tests"
	@echo ""
	@echo "Postgres (integration tests):"
	@echo "  postgres-up          docker compose up (port 5433, user postgres / wostest)"
	@echo "  postgres-down        docker compose down"
	@echo "  export DATABASE_URL=postgres://postgres:wostest@127.0.0.1:5433/postgres"
	@echo ""
	@echo "Restate (WS-094 Phase 4):"
	@echo "  restate-ingress-smoke  Docker Restate + worker + ignored ingress test (needs Docker; worker probe uses nc or bash /dev/tcp)"
	@echo ""
	@echo "Sibling repos (extracted 2026-05-04):"
	@echo "  Studio (Authoring) lives in policy-studio/ — (cd ../policy-studio && cargo build --workspace)"
	@echo "  Case Portal lives in case-portal/ — (cd ../case-portal && npm install && npm run build)"

# Build
build: rust-build

rust-build:
	$(CARGO) build --workspace

# Test
test: rust-test python-test

# Rust + Python schema tests only (no Studio Vitest). Use for a faster inner
# loop; root CI `make test-wos-spec` intentionally stays aligned with full `test`.
test-core: rust-test python-test

rust-test:
	@echo "Running Rust workspace tests (nextest)."
	@echo "Note: discovery for large integration binaries can take a while before PASS lines appear."
	$(CARGO) nextest run --workspace --no-fail-fast
	@echo "Running wos-conformance Restate parity slice (feature restate-tests)."
	$(CARGO) nextest run -p wos-conformance --features restate-tests --test r6_restate_conformance_slice --no-fail-fast

COMPOSE_POSTGRES := docker compose -f docker-compose.postgres.yml

postgres-up:
	$(COMPOSE_POSTGRES) up -d
	@echo "Postgres listening on 127.0.0.1:5433"
	@echo "export DATABASE_URL='postgres://postgres:wostest@127.0.0.1:5433/postgres'"
	@echo "export WOS_POSTGRES_TEST_URL='postgres://postgres:wostest@127.0.0.1:5433/postgres'"

postgres-down:
	$(COMPOSE_POSTGRES) down

restate-ingress-smoke:
	bash scripts/restate_ingress_smoke.sh

python-test:
	$(PYTEST) tests/schemas -q

# Studio (Authoring) and Case Portal extracted to sibling repos on 2026-05-04.
# Their build/test/lint/clean live in those repos' own Makefiles.

# Lint & Check
lint: rust-check check-openapi-parity

rust-check:
	$(CARGO) check --workspace

# Wave 7 Finding A1 — OpenAPI dual-surface parity gate.
# Asserts every response code in the utoipa-emitted registry OpenAPI is
# mirrored in the hand-maintained broader OpenAPI for overlapping
# (METHOD, PATH) pairs, and that every registry-wired operation is
# declared in the broader contract. See script header for prior-art notes
# (oasdiff alternative) and assertion shape.
check-openapi-parity:
	python3 scripts/check_openapi_dual_surface_parity.py

# Clean
clean: rust-clean

rust-clean:
	$(CARGO) clean
