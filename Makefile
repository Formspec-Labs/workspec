# WOS (Workflow Orchestration Standard) Makefile

# Variables
CARGO = cargo
PYTHON = python3
# Plugin autoload off: the schema tests need only pytest + jsonschema (as in CI);
# a globally installed pytest11 plugin with a broken dependency otherwise aborts collection.
PYTEST = PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 $(PYTHON) -m pytest
# The uv that fills and then reads the wos-mcp official client's offline cache
# (crates/wos-mcp/tests/actual_client.rs reads WOS_MCP_UV); uv versions its cache buckets,
# so both use one binary, at the version uv.toml pins (root CI's setup-uv installs it from
# there). A WOS_MCP_UV (default: `uv` on PATH) at any other version is replaced by the
# digest-checked install scripts/install-uv.sh makes under .tools/ (wos-mcp-client-bootstrap).
WOS_MCP_UV_VERSION := $(shell sed -n 's/^required-version *= *"==\([^"]*\)"$$/\1/p' uv.toml)
WOS_MCP_UV_LOCAL := $(CURDIR)/.tools/uv-$(WOS_MCP_UV_VERSION)/uv
WOS_MCP_UV ?= uv
ifneq ($(shell $(WOS_MCP_UV) --version 2>/dev/null | cut -d' ' -f2),$(WOS_MCP_UV_VERSION))
override WOS_MCP_UV := $(WOS_MCP_UV_LOCAL)
endif
export WOS_MCP_UV

# Targets
.PHONY: all build test test-core lint clean help \
	rust-build rust-test rust-check \
	python-test wos-mcp-client-bootstrap \
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
	@echo "  lint                 Run all linters and checks (rust-check)"
	@echo "  clean                Remove build artifacts"
	@echo ""
	@echo "Parent Rust Targets (wos-spec workspace):"
	@echo "  rust-build           Build all parent Rust crates"
	@echo "  rust-test            Run all parent Rust tests (runs wos-mcp-client-bootstrap first)"
	@echo "  wos-mcp-client-bootstrap  Fill WOS_MCP_UV's cache for the wos-mcp official client (network on first run)"
	@echo "  rust-check           Run cargo check on the parent workspace"
	@echo ""
	@echo "Parent Python Targets:"
	@echo "  python-test          Run Python schema-conformance tests and check-* gates (as CI)"
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

# Fills the uv cache from tests/support/official_client.py.lock; the test then runs it offline.
wos-mcp-client-bootstrap:
ifeq ($(WOS_MCP_UV),$(WOS_MCP_UV_LOCAL))
	scripts/install-uv.sh $(dir $(WOS_MCP_UV_LOCAL))
endif
	$(WOS_MCP_UV) sync --script crates/wos-mcp/tests/support/official_client.py --locked

rust-test: wos-mcp-client-bootstrap
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

# The one list of Python gates: .github/workflows/schema-regression.yml runs this target.
python-test:
	$(PYTEST) tests/schemas -q
	$(PYTHON) scripts/check-canonical-seams.py
	$(PYTHON) scripts/check-recordkind-parity.py

# Studio (Authoring) and Case Portal extracted to sibling repos on 2026-05-04.
# Their build/test/lint/clean live in those repos' own Makefiles.

# Lint & Check
lint: rust-check

rust-check:
	$(CARGO) check --workspace

# Clean
clean: rust-clean

rust-clean:
	$(CARGO) clean
