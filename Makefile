# =============================================================================
# SSO-1 Makefile
# =============================================================================
# Primary build and development automation for the SSO-1 protocol.
# =============================================================================

.PHONY: all build clean test help
.PHONY: build-onchain build-offchain
.PHONY: test-onchain test-offchain test-integration
.PHONY: deploy-devnet deploy-mainnet
.PHONY: docker-build docker-push
.PHONY: lint fmt check

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

ANCHOR_VERSION := 0.30.0
SOLANA_VERSION := 1.18.0
PYTHON_VERSION := 3.11

ONCHAIN_DIR := oracle/onchain
OFFCHAIN_DIR := oracle/offchain/function

DOCKER_REGISTRY := ghcr.io/your-org
DOCKER_IMAGE := sso-1-function
DOCKER_TAG := latest

# -----------------------------------------------------------------------------
# Default Target
# -----------------------------------------------------------------------------

all: build

help:
	@echo "SSO-1 Build System"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Build Targets:"
	@echo "  build           Build all components"
	@echo "  build-onchain   Build Anchor program"
	@echo "  build-offchain  Build Switchboard function container"
	@echo "  clean           Clean all build artifacts"
	@echo ""
	@echo "Test Targets:"
	@echo "  test            Run all tests"
	@echo "  test-onchain    Run Anchor program tests"
	@echo "  test-offchain   Run Python function tests"
	@echo "  test-integration Run integration tests (requires devnet)"
	@echo ""
	@echo "Deployment Targets:"
	@echo "  deploy-devnet   Deploy to Solana devnet"
	@echo "  deploy-mainnet  Deploy to Solana mainnet (requires confirmation)"
	@echo ""
	@echo "Docker Targets:"
	@echo "  docker-build    Build function Docker image"
	@echo "  docker-push     Push image to registry"
	@echo ""
	@echo "Code Quality:"
	@echo "  lint            Run linters"
	@echo "  fmt             Format code"
	@echo "  check           Run all checks (lint + test)"

# -----------------------------------------------------------------------------
# Build Targets
# -----------------------------------------------------------------------------

build: build-onchain build-offchain
	@echo "✓ All components built successfully"

build-onchain:
	@echo "Building on-chain program..."
	cd $(ONCHAIN_DIR) && anchor build
	@echo "✓ On-chain program built"

build-offchain:
	@echo "Building off-chain function..."
	cd $(OFFCHAIN_DIR) && docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .
	@echo "✓ Off-chain function built"

clean:
	@echo "Cleaning build artifacts..."
	cd $(ONCHAIN_DIR) && anchor clean || true
	rm -rf $(ONCHAIN_DIR)/target
	rm -rf $(OFFCHAIN_DIR)/__pycache__
	rm -rf $(OFFCHAIN_DIR)/.pytest_cache
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	@echo "✓ Clean complete"

# -----------------------------------------------------------------------------
# Test Targets
# -----------------------------------------------------------------------------

test: test-onchain test-offchain
	@echo "✓ All tests passed"

test-onchain:
	@echo "Running on-chain tests..."
	cd $(ONCHAIN_DIR) && anchor test
	@echo "✓ On-chain tests passed"

test-offchain:
	@echo "Running off-chain tests..."
	cd $(OFFCHAIN_DIR) && python -m pytest ../../../tests/ -v
	@echo "✓ Off-chain tests passed"

test-integration:
	@echo "Running integration tests on devnet..."
	@echo "WARNING: This requires devnet SOL and deployed programs"
	./scripts/devnet/run_integration_tests.sh
	@echo "✓ Integration tests passed"

# -----------------------------------------------------------------------------
# Deployment Targets
# -----------------------------------------------------------------------------

deploy-devnet:
	@echo "Deploying to Solana devnet..."
	cd $(ONCHAIN_DIR) && anchor deploy --provider.cluster devnet
	@echo "✓ Deployed to devnet"
	@echo "Update .env with the new program ID"

deploy-mainnet:
	@echo "WARNING: You are about to deploy to mainnet!"
	@echo "This action cannot be undone."
	@read -p "Type 'DEPLOY' to confirm: " confirm && [ "$$confirm" = "DEPLOY" ]
	cd $(ONCHAIN_DIR) && anchor deploy --provider.cluster mainnet
	@echo "✓ Deployed to mainnet"

# -----------------------------------------------------------------------------
# Docker Targets
# -----------------------------------------------------------------------------

docker-build:
	@echo "Building Docker image..."
	cd $(OFFCHAIN_DIR) && docker build \
		-t $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG) \
		-t $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$$(git rev-parse --short HEAD) \
		.
	@echo "✓ Docker image built"

docker-push:
	@echo "Pushing Docker image to registry..."
	docker push $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
	docker push $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$$(git rev-parse --short HEAD)
	@echo "✓ Docker image pushed"

# -----------------------------------------------------------------------------
# Code Quality Targets
# -----------------------------------------------------------------------------

lint:
	@echo "Running linters..."
	cd $(ONCHAIN_DIR) && cargo clippy --all-targets -- -D warnings
	cd $(OFFCHAIN_DIR) && python -m ruff check .
	@echo "✓ Lint passed"

fmt:
	@echo "Formatting code..."
	cd $(ONCHAIN_DIR) && cargo fmt
	cd $(OFFCHAIN_DIR) && python -m ruff format .
	@echo "✓ Format complete"

check: lint test
	@echo "✓ All checks passed"

# -----------------------------------------------------------------------------
# Development Helpers
# -----------------------------------------------------------------------------

.PHONY: setup-dev keys logs

setup-dev:
	@echo "Setting up development environment..."
	@echo "Installing Rust dependencies..."
	cd $(ONCHAIN_DIR) && cargo fetch
	@echo "Installing Python dependencies..."
	cd $(OFFCHAIN_DIR) && pip install -r requirements.txt
	@echo "Copying environment template..."
	cp -n .env.example .env || true
	@echo "✓ Development environment ready"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Edit .env with your configuration"
	@echo "  2. Run 'make build' to build all components"
	@echo "  3. Run 'make test' to run tests"

keys:
	@echo "Generating development keypairs..."
	mkdir -p keypairs
	solana-keygen new --no-bip39-passphrase -o keypairs/authority.json --force
	@echo "✓ Keypairs generated in ./keypairs/"
	@echo "WARNING: These are for development only. Never use in production."

logs:
	@echo "Tailing Solana logs..."
	solana logs --url devnet
