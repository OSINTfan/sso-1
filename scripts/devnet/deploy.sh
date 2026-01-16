#!/usr/bin/env bash
# =============================================================================
# SSO-1 Devnet Deployment Script
# =============================================================================
# Deploys the SSO-1 protocol to Solana devnet.
#
# Prerequisites:
# - Solana CLI installed and configured
# - Anchor CLI installed
# - Devnet SOL in deployer wallet
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
ONCHAIN_DIR="${ROOT_DIR}/oracle/onchain"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    log_info "=========================================="
    log_info "SSO-1 Devnet Deployment"
    log_info "=========================================="
    
    # Ensure devnet
    log_info "Configuring Solana CLI for devnet..."
    solana config set --url devnet
    
    # Check balance
    BALANCE=$(solana balance | awk '{print $1}')
    log_info "Wallet balance: ${BALANCE} SOL"
    
    if (( $(echo "$BALANCE < 2" | bc -l) )); then
        log_warn "Low balance, requesting airdrop..."
        solana airdrop 2 || log_warn "Airdrop may have failed, continuing..."
        sleep 5
    fi
    
    # Build
    log_info "Building program..."
    cd "${ONCHAIN_DIR}"
    anchor build
    
    # Deploy
    log_info "Deploying to devnet..."
    anchor deploy --provider.cluster devnet
    
    # Get program ID
    PROGRAM_ID=$(solana address -k target/deploy/sso_signal-keypair.json 2>/dev/null || echo "unknown")
    
    log_info "=========================================="
    log_info "Deployment complete!"
    log_info "Program ID: ${PROGRAM_ID}"
    log_info "=========================================="
    log_info ""
    log_info "Next steps:"
    log_info "1. Update .env with SSO_SIGNAL_PROGRAM_ID=${PROGRAM_ID}"
    log_info "2. Update oracle/onchain/Anchor.toml with new program ID"
    log_info "3. Run integration tests: make test-integration"
}

main "$@"
