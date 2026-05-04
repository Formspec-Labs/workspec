# WOS (Workflow Orchestration Standard) Makefile

# Variables
CARGO = cargo
PYTEST = python3 -m pytest
NPM = npm
CASE_PORTAL_DIR = case-portal
STUDIO_DIR = studio

# Targets
.PHONY: all build test test-core lint clean help \
	rust-build rust-test rust-check \
	python-test \
	case-portal-build case-portal-test case-portal-lint case-portal-clean case-portal-install case-portal-types \
	studio-test studio-check studio-build studio-clean \
	postgres-up postgres-down \
	restate-ingress-smoke

# Default target: build and test everything (parent + studio + case-portal)
all: build test

help:
	@echo "WOS (Workflow Orchestration Standard) Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Primary Targets:"
	@echo "  all                  Build and test everything"
	@echo "  build                Build parent Rust workspace and Case Portal frontend"
	@echo "  test                 Run all tests (parent Rust, parent Python, Studio, Case Portal)"
	@echo "  test-core            Run parent Rust + Python tests only (skip Studio + Case Portal)"
	@echo "  lint                 Run all linters and checks"
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
	@echo "Studio (Authoring) Targets — separate workspace under studio/:"
	@echo "  studio-test          Run Studio Python schema regression tests"
	@echo "  studio-check         Run cargo check on the Studio workspace (once Wave 0.2 lands)"
	@echo "  studio-build         Build Studio Rust crates (once Wave 0.2 lands)"
	@echo "  studio-clean         Remove Studio build artifacts"
	@echo ""
	@echo "Case Portal (Frontend) Targets:"
	@echo "  case-portal-build    Build Case Portal frontend (formerly 'studio-build')"
	@echo "  case-portal-test     Run Case Portal vitest suite"
	@echo "  case-portal-lint     Run Case Portal type checks"
	@echo "  case-portal-types    Generate WOS types for Case Portal"
	@echo "  case-portal-install  Install Case Portal dependencies"

# Build
build: rust-build case-portal-build

rust-build:
	$(CARGO) build --workspace

case-portal-build: case-portal-install
	cd $(CASE_PORTAL_DIR) && $(NPM) run build

# Test
test: rust-test python-test studio-test case-portal-test

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

case-portal-test: case-portal-install
	cd $(CASE_PORTAL_DIR) && $(NPM) run test

# Studio (Authoring)
# Python schema regression suite always runs; cargo targets are gated on
# studio/Cargo.toml existing (Wave 0.2 lands the workspace).
studio-test:
	$(PYTEST) $(STUDIO_DIR)/tests/schemas -q

studio-check:
	@if [ -f $(STUDIO_DIR)/Cargo.toml ]; then \
		cd $(STUDIO_DIR) && $(CARGO) check --workspace; \
	else \
		echo "skip: $(STUDIO_DIR)/Cargo.toml does not exist yet (Wave 0.2 pending)"; \
	fi

studio-build:
	@if [ -f $(STUDIO_DIR)/Cargo.toml ]; then \
		cd $(STUDIO_DIR) && $(CARGO) build --workspace; \
	else \
		echo "skip: $(STUDIO_DIR)/Cargo.toml does not exist yet (Wave 0.2 pending)"; \
	fi

studio-clean:
	@if [ -f $(STUDIO_DIR)/Cargo.toml ]; then \
		cd $(STUDIO_DIR) && $(CARGO) clean; \
	fi

# Lint & Check
lint: rust-check studio-check case-portal-lint

rust-check:
	$(CARGO) check --workspace

case-portal-lint: case-portal-install
	cd $(CASE_PORTAL_DIR) && $(NPM) run lint

case-portal-types: case-portal-install
	cd $(CASE_PORTAL_DIR) && $(NPM) run types:gen

# Clean
clean: rust-clean studio-clean case-portal-clean

rust-clean:
	$(CARGO) clean

case-portal-clean:
	cd $(CASE_PORTAL_DIR) && $(NPM) run clean

# Setup
case-portal-install:
	cd $(CASE_PORTAL_DIR) && $(NPM) install
