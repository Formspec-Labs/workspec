# WOS (Workflow Orchestration Standard) Makefile

# Variables
CARGO = cargo
PYTEST = python3 -m pytest
NPM = npm
STUDIO_DIR = studio

# Targets
.PHONY: all build test test-core lint clean help \
	rust-build rust-test rust-check \
	python-test \
	studio-build studio-test studio-lint studio-clean studio-install studio-types \
	postgres-up postgres-down \
	restate-ingress-smoke

# Default target: build and test everything
all: build test

help:
	@echo "WOS (Workflow Orchestration Standard) Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Primary Targets:"
	@echo "  all           Build and test everything"
	@echo "  build         Build Rust workspace and Studio frontend"
	@echo "  test          Run all tests (Rust, Python, Studio)"
	@echo "  test-core     Run Rust + Python tests (skip Studio Vitest)"
	@echo "  lint          Run all linters and checks"
	@echo "  clean         Remove build artifacts"
	@echo ""
	@echo "Rust Targets:"
	@echo "  rust-build    Build all Rust crates"
	@echo "  rust-test     Run all Rust tests"
	@echo "  rust-check    Run cargo check on the workspace"
	@echo ""
	@echo "Postgres (integration tests):"
	@echo "  postgres-up   docker compose up (port 5433, user postgres / wostest)"
	@echo "  postgres-down docker compose down"
	@echo "  export DATABASE_URL=postgres://postgres:wostest@127.0.0.1:5433/postgres"
	@echo ""
	@echo "Restate (WS-094 Phase 4):"
	@echo "  restate-ingress-smoke  Docker Restate + worker + ignored ingress test (needs Docker; worker probe uses nc or bash /dev/tcp)"
	@echo ""
	@echo "Python Targets:"
	@echo "  python-test   Run Python schema-conformance tests"
	@echo ""
	@echo "Studio (Frontend) Targets:"
	@echo "  studio-build  Build Studio frontend"
	@echo "  studio-test   Run Studio vitest suite"
	@echo "  studio-lint   Run Studio type checks"
	@echo "  studio-types  Generate WOS types for Studio"
	@echo "  studio-install Install Studio dependencies"

# Build
build: rust-build studio-build

rust-build:
	$(CARGO) build --workspace

studio-build: studio-install
	cd $(STUDIO_DIR) && $(NPM) run build

# Test
test: rust-test python-test studio-test

# Rust + Python schema tests only (no Studio Vitest). Use for a faster inner
# loop; root CI `make test-wos-spec` intentionally stays aligned with full `test`.
test-core: rust-test python-test

rust-test:
	@echo "Running Rust workspace tests (nextest)."
	@echo "Note: discovery for large integration binaries (e.g. wos-server) can take a while before PASS lines appear."
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

studio-test: studio-install
	cd $(STUDIO_DIR) && $(NPM) run test

# Lint & Check
lint: rust-check studio-lint

rust-check:
	$(CARGO) check --workspace

studio-lint: studio-install
	cd $(STUDIO_DIR) && $(NPM) run lint

studio-types: studio-install
	cd $(STUDIO_DIR) && $(NPM) run types:gen

# Clean
clean: rust-clean studio-clean

rust-clean:
	$(CARGO) clean

studio-clean:
	cd $(STUDIO_DIR) && $(NPM) run clean

# Setup
studio-install:
	cd $(STUDIO_DIR) && $(NPM) install
