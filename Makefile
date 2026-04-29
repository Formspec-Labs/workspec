# WOS (Workflow Orchestration Standard) Makefile

# Variables
CARGO = cargo
PYTEST = python3 -m pytest
NPM = npm
STUDIO_DIR = studio

# Targets
.PHONY: all build test lint clean help \
	rust-build rust-test rust-check \
	python-test \
	studio-build studio-test studio-lint studio-clean studio-install studio-types

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
	@echo "  lint          Run all linters and checks"
	@echo "  clean         Remove build artifacts"
	@echo ""
	@echo "Rust Targets:"
	@echo "  rust-build    Build all Rust crates"
	@echo "  rust-test     Run all Rust tests"
	@echo "  rust-check    Run cargo check on the workspace"
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

rust-test:
	$(CARGO) nextest run --workspace

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
