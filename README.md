Repository Archived — Superseded by SSO-1 v1.3

Status: Archived (Read-only)
Reason: Architectural supersession

This repository represents an early experimental implementation of SSO-1 that was built before key architectural constraints were fully understood and formalized.

Since then, the protocol has undergone a ground-up redesign, culminating in SSO-1 v1.3, which introduces a materially different trust model and execution architecture:

A two-phase verification model separating one-time hardware attestation from high-frequency signal submission

Amortized verification via zkVM-wrapped SEV-SNP attestation

A strict Ed25519-only hot path compatible with Solana’s transaction size and compute limits

Slot-relative validity and explicit economic backstops

The design assumptions embedded in this repository (single-phase verification, direct hardware assumptions, or earlier trust abstractions) are no longer aligned with the current specification and should not be used as a reference for production systems.

Why archive instead of rewrite?

This repository is archived intentionally to:

Preserve historical context and design evolution

Avoid ambiguity between incompatible protocol versions

Prevent accidental reuse of outdated assumptions

A new repository, built from first principles and strictly aligned with the SSO-1 v1.3 specification, is now the canonical implementation.

Canonical resources

Specification: SSO-1 v1.3 (January 2026)

Current implementation: [coming soon - will be linked here]

Security model: See updated threat model in the new repository [coming soon - will be linked here]

This repository is retained for reference only.
No further development, maintenance, or security review will occur here.





//////////////////////////////////////////////////////////////////

Old readme below.





# SSO-1: Standardized Verifiable Signal Oracle

> **SSO-1 is a signal oracle infrastructure primitive, not a trading or portfolio management system.**

## Overview

SSO-1 defines a protocol for publishing, verifying, and consuming cryptographically attested trading signals on Solana. It leverages Switchboard V3 On-Demand oracles and AMD SEV-SNP Trusted Execution Environments (TEE) to provide:

- **Verifiable Signal Provenance**: All signals are generated inside a TEE with cryptographic attestation
- **Slot-Relative Validity**: Signal validity is enforced via Solana slot semantics, not wall-clock time
- **Pull-Based Architecture**: Consumers request updates Just-In-Time, avoiding stale data
- **Hard Separation of Concerns**: Objective `MarketContext` is strictly separated from subjective `SignalAssessment`

## Specification Version

This implementation targets **SSO-1 Specification v1.2 (January 2026)**.

See [SPEC.md](./SPEC.md) for the normative protocol text.

## Intended Consumers

- **Vaults**: Automated DeFi vaults requiring signal inputs
- **Trading Bots**: Algorithmic trading systems
- **Autonomous Agents**: AI/ML agents requiring market signals
- **Protocol Integrations**: Other protocols building on verifiable signals

## Architecture

```
┌─────────────┐     ┌─────────────────────┐     ┌─────────────┐     ┌──────────────┐
│  Consumer   │────▶│ Switchboard Function │────▶│  TEE (SNP)  │────▶│  On-Chain    │
│  (Vault/Bot)│     │   (Pull Request)     │     │  Execution  │     │  Verification│
└─────────────┘     └─────────────────────┘     └─────────────┘     └──────────────┘
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed data flow documentation.

## Repository Structure

```
sso-1/
├── oracle/
│   ├── offchain/       # Switchboard Function (Python, TEE)
│   └── onchain/        # Anchor program (Rust)
├── schemas/            # Signal schema documentation
├── config/             # Configuration templates
├── scripts/            # Development and deployment scripts
└── tests/              # Integration tests
```

## Quick Start

```bash
# Clone the repository
git clone https://github.com/your-org/sso-1.git
cd sso-1

# Copy environment template
cp .env.example .env

# Build on-chain program
make build-onchain

# Build off-chain function container
make build-offchain

# Run integration tests (devnet)
make test-integration
```

## Non-Goals

SSO-1 explicitly does **NOT** provide:

- Alpha generation or prediction logic
- Trading strategies or portfolio management
- Token issuance or incentive mechanisms
- Push-based signal delivery
- Wall-clock time assumptions
- Business logic beyond validation and persistence

## Trust Model

| Component | Trust Assumption |
|-----------|------------------|
| TEE (AMD SEV-SNP) | Hardware root of trust for signal computation |
| Switchboard V3 | Decentralized function execution and attestation |
| On-Chain Program | Trustless verification of TEE attestations |
| Data Sources | Configurable; multiple providers for resilience |

## License

Apache 2.0 - See [LICENSE](./LICENSE)

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

---

**Specification**: v1.2 (January 2026)  
**Network**: Solana (Alpenglow Era)  
**Oracle Framework**: Switchboard V3 On-Demand

