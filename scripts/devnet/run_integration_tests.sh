#!/usr/bin/env bash
# =============================================================================
# SSO-1 Devnet Integration Test Runner
# =============================================================================
# This script runs integration tests against the Solana devnet.
#
# Prerequisites:
# - Solana CLI installed and configured
# - Anchor CLI installed
# - Devnet SOL in test wallet
# - SSO-1 program deployed to devnet
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
ONCHAIN_DIR="${ROOT_DIR}/oracle/onchain"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# -----------------------------------------------------------------------------
# Utility Functions
# -----------------------------------------------------------------------------

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "$1 is not installed"
        exit 1
    fi
}

# -----------------------------------------------------------------------------
# Pre-flight Checks
# -----------------------------------------------------------------------------

preflight_checks() {
    log_info "Running pre-flight checks..."
    
    # Check required commands
    check_command "solana"
    check_command "anchor"
    
    # Check Solana configuration
    CLUSTER=$(solana config get | grep "RPC URL" | awk '{print $3}')
    if [[ ! "$CLUSTER" =~ "devnet" ]]; then
        log_warn "Solana CLI is not configured for devnet"
        log_info "Switching to devnet..."
        solana config set --url devnet
    fi
    
    # Check wallet balance
    BALANCE=$(solana balance | awk '{print $1}')
    if (( $(echo "$BALANCE < 1" | bc -l) )); then
        log_error "Insufficient devnet SOL balance: $BALANCE"
        log_info "Request airdrop with: solana airdrop 2"
        exit 1
    fi
    
    log_info "Pre-flight checks passed"
}

# -----------------------------------------------------------------------------
# Build Programs
# -----------------------------------------------------------------------------

build_programs() {
    log_info "Building on-chain programs..."
    
    cd "${ONCHAIN_DIR}"
    anchor build
    
    log_info "Build complete"
}

# -----------------------------------------------------------------------------
# Deploy to Devnet
# -----------------------------------------------------------------------------

deploy_devnet() {
    log_info "Deploying to devnet..."
    
    cd "${ONCHAIN_DIR}"
    anchor deploy --provider.cluster devnet
    
    # Extract program ID
    PROGRAM_ID=$(solana address -k target/deploy/sso_signal-keypair.json)
    log_info "Deployed program ID: ${PROGRAM_ID}"
    
    echo "${PROGRAM_ID}"
}

# -----------------------------------------------------------------------------
# Run Tests
# -----------------------------------------------------------------------------

run_tests() {
    log_info "Running integration tests..."
    
    cd "${ONCHAIN_DIR}"
    
    # Run Anchor tests against devnet
    anchor test --provider.cluster devnet --skip-local-validator
    
    log_info "Tests complete"
}

# -----------------------------------------------------------------------------
# Cleanup
# -----------------------------------------------------------------------------

cleanup() {
    log_info "Cleaning up..."
    
    # TODO: Add cleanup logic
    # - Close test accounts
    # - Reclaim rent
    
    log_info "Cleanup complete"
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    log_info "=========================================="
    log_info "SSO-1 Devnet Integration Tests"
    log_info "=========================================="
    
    preflight_checks
    
    # Parse arguments
    SKIP_BUILD=false
    SKIP_DEPLOY=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-build)
                SKIP_BUILD=true
                shift
                ;;
            --skip-deploy)
                SKIP_DEPLOY=true
                shift
                ;;
            --help)
                echo "Usage: $0 [--skip-build] [--skip-deploy]"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Build if not skipped
    if [ "$SKIP_BUILD" = false ]; then
        build_programs
    fi
    
    # Deploy if not skipped
    if [ "$SKIP_DEPLOY" = false ]; then
        deploy_devnet
    fi
    
    # Run tests
    run_tests
    
    # Cleanup
    cleanup
    
    log_info "=========================================="
    log_info "Integration tests completed successfully!"
    log_info "=========================================="
}

# Run main function
main "$@"
