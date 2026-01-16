# SSO-1: Standardized Verifiable Signal Oracle

**Specification Version**: 1.2  
**Date**: January 2026  
**Status**: Normative Protocol Text

---

## Abstract

SSO-1 defines a standardized protocol for publishing, verifying, and consuming cryptographically attested trading signals on the Solana blockchain. The protocol leverages Trusted Execution Environments (TEE) and the Switchboard V3 On-Demand oracle framework to provide verifiable signal provenance with slot-relative validity semantics.

---

## 1. Introduction

### 1.1 Purpose

This specification establishes the normative requirements for SSO-1 compliant signal oracles operating on Solana. SSO-1 is an infrastructure primitive designed to be consumed by vaults, bots, agents, and other automated systems requiring verifiable market signals.

### 1.2 Scope

SSO-1 addresses:

- Signal data structure and encoding
- Trusted Execution Environment requirements
- On-chain verification and persistence
- Validity semantics and expiration
- Consumer integration patterns

SSO-1 does NOT address:

- Signal generation algorithms or models
- Trading strategies or portfolio construction
- Token economics or incentive mechanisms
- Data source selection or aggregation logic

### 1.3 Terminology

| Term | Definition |
|------|------------|
| **Signal** | A structured assessment of market conditions with associated confidence |
| **MarketContext** | Objective, observable market data at a specific slot |
| **SignalAssessment** | Subjective interpretation derived from MarketContext |
| **TEE** | Trusted Execution Environment providing hardware-attested computation |
| **Slot** | Solana's fundamental unit of time (~400ms) |
| **Valid Until Slot** | The slot after which a signal MUST be considered stale |

---

## 2. Protocol Architecture

### 2.1 System Components

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           SSO-1 Protocol                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐    ┌───────────────────┐    ┌────────────────────┐   │
│  │   Consumer   │───▶│ Switchboard V3    │───▶│  TEE Enclave       │   │
│  │              │    │ On-Demand Function │    │  (AMD SEV-SNP)     │   │
│  └──────────────┘    └───────────────────┘    └────────────────────┘   │
│         ▲                                              │                │
│         │            ┌───────────────────┐             │                │
│         └────────────│  On-Chain Program │◀────────────┘                │
│                      │  (Verification)   │                              │
│                      └───────────────────┘                              │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow

1. **Consumer** initiates a pull request via Switchboard Function
2. **Switchboard** dispatches execution to TEE-enabled infrastructure
3. **TEE Enclave** computes signal within attested environment
4. **TEE** produces signed attestation with signal payload
5. **On-Chain Program** verifies attestation and persists signal
6. **Consumer** reads verified signal from on-chain account

### 2.3 Trust Boundaries

| Boundary | Trust Assumption | Verification Method |
|----------|------------------|---------------------|
| TEE → On-Chain | Hardware attestation | AMD SEV-SNP attestation verification |
| Data Source → TEE | Configurable trust | Multi-source aggregation (OPTIONAL) |
| On-Chain → Consumer | Blockchain consensus | Solana transaction finality |

---

## 3. Data Structures

### 3.1 MarketContext

MarketContext represents objective, observable market state at a specific slot.

```
MarketContext {
    slot: u64                      // Solana slot at observation
    asset_pair: [u8; 32]           // Normalized asset pair identifier
    price: u64                     // Price in base units (scaled by 10^9)
    volume_24h: u64                // 24h volume in quote units
    volatility_1h: u64             // 1-hour realized volatility (scaled by 10^6)
    liquidity_depth: u64           // Available liquidity within 2% (quote units)
    source_bitmap: u64             // Bitmap of contributing data sources
    source_count: u8               // Number of sources aggregated
}
```

**Requirements**:

- MUST be derived from observable market data only
- MUST NOT contain subjective assessments
- MUST include slot of observation
- SHOULD aggregate multiple data sources when available

### 3.2 SignalAssessment

SignalAssessment represents a subjective interpretation of MarketContext.

```
SignalAssessment {
    signal_type: SignalType        // Enumerated signal type
    direction: Direction           // Long, Short, Neutral
    magnitude: u8                  // 0-100 normalized magnitude
    confidence: u8                 // 0-100 confidence score
    valid_from_slot: u64           // Slot when signal becomes valid
    valid_until_slot: u64          // Slot when signal expires (REQUIRED)
    model_version: [u8; 8]         // Model identifier for reproducibility
}
```

**Requirements**:

- MUST reference a specific MarketContext
- MUST specify valid_until_slot
- MUST be generated within TEE
- SHOULD include model versioning

### 3.3 TeeReceipt

TeeReceipt captures the cryptographic attestation from the TEE.

```
TeeReceipt {
    enclave_signer: Pubkey         // TEE enclave public key
    attestation_hash: [u8; 32]     // SHA-256 of attestation document
    mr_enclave: [u8; 32]           // Measurement of enclave code
    timestamp_slot: u64            // Slot at attestation generation
    platform_version: u16          // TEE platform version
}
```

**Requirements**:

- MUST contain verifiable enclave measurement
- MUST be validated against known-good mr_enclave values
- MUST be generated by AMD SEV-SNP or equivalent

### 3.4 SignalAccount

SignalAccount is the on-chain account storing verified signals.

```
SignalAccount {
    version: u8                    // Account version for migrations
    authority: Pubkey              // Update authority
    asset_pair: [u8; 32]           // Asset pair this account tracks
    market_context: MarketContext  // Latest objective context
    signal_assessment: SignalAssessment // Latest subjective assessment
    tee_receipt: TeeReceipt        // Attestation proof
    update_count: u64              // Total updates for sequencing
    last_update_slot: u64          // Slot of last update
}
```

---

## 4. Protocol Operations

### 4.1 Signal Update Flow

```
Consumer                    Switchboard              TEE                 On-Chain
    │                           │                     │                      │
    │── Request Signal ────────▶│                     │                      │
    │                           │── Execute ─────────▶│                      │
    │                           │                     │── Compute Signal     │
    │                           │                     │── Generate Attestation│
    │                           │◀── Result + Attest ─│                      │
    │                           │                     │                      │
    │                           │── Submit Tx ───────────────────────────────▶│
    │                           │                                             │
    │                           │                     │◀── Verify Attestation │
    │                           │                     │◀── Validate Slots     │
    │                           │                     │◀── Persist Signal     │
    │                           │                                             │
    │◀── Read Signal ────────────────────────────────────────────────────────│
    │                                                                         │
```

### 4.2 Validity Enforcement

**Slot-Relative Validity** is a first-class protocol invariant:

1. On update: `current_slot <= valid_until_slot` MUST be true
2. On read: Consumer SHOULD verify `current_slot <= valid_until_slot`
3. Expired signals MUST NOT be used for decisions

**Implementation**:

```rust
// On-chain validation (REQUIRED)
require!(
    clock.slot <= signal_assessment.valid_until_slot,
    ErrorCode::SignalExpired
);
```

### 4.3 Update Instruction

The `update_signal` instruction:

1. Accepts MarketContext, SignalAssessment, and TeeReceipt
2. Verifies TEE attestation against known mr_enclave
3. Validates slot constraints
4. Persists to SignalAccount
5. Emits update event

---

## 5. TEE Requirements

### 5.1 Platform

- AMD SEV-SNP is the REQUIRED platform for v1.2
- Intel TDX support is RESERVED for future versions
- Enclave measurement (mr_enclave) MUST be published and auditable

### 5.2 Attestation

- Attestation MUST be generated for each signal computation
- Attestation MUST include enclave measurement
- Attestation MUST be verifiable on-chain or via Switchboard

### 5.3 Security Properties

The TEE MUST provide:

- **Confidentiality**: Model weights/logic protected from host
- **Integrity**: Computation cannot be tampered with
- **Attestation**: Cryptographic proof of correct execution

---

## 6. Switchboard Integration

### 6.1 Function Requirements

- Function MUST be registered with Switchboard V3
- Function MUST execute within TEE enclave
- Function MUST produce attestation with result

### 6.2 On-Demand Pattern

SSO-1 uses the **pull-based, Just-In-Time** pattern:

- Consumers trigger updates as needed
- No continuous push updates
- Reduces costs and stale data

---

## 7. Versioning

### 7.1 Specification Versioning

- MAJOR: Breaking changes to data structures or verification
- MINOR: Backward-compatible additions
- This document: v1.2

### 7.2 Account Versioning

- SignalAccount includes version field
- Migrations MUST be explicit and auditable

---

## 8. Security Considerations

### 8.1 Adversarial Model

Assume:

- Consumers may attempt to use stale signals
- Data sources may provide incorrect data
- Operators may attempt to bypass TEE

### 8.2 Mitigations

- Slot-relative validity prevents stale signal usage
- Multi-source aggregation reduces single-source risk
- TEE attestation prevents execution tampering

---

## 9. Conformance

An implementation is SSO-1 v1.2 conformant if it:

1. Implements all REQUIRED data structures
2. Enforces slot-relative validity on-chain
3. Verifies TEE attestations
4. Separates MarketContext from SignalAssessment
5. Uses pull-based update pattern

---

## Appendix A: Signal Types

| Value | Name | Description |
|-------|------|-------------|
| 0 | MOMENTUM | Trend-following signal |
| 1 | MEAN_REVERSION | Mean reversion signal |
| 2 | VOLATILITY | Volatility regime signal |
| 3 | LIQUIDITY | Liquidity condition signal |
| 4-255 | RESERVED | Reserved for future use |

## Appendix B: Direction Values

| Value | Name | Description |
|-------|------|-------------|
| 0 | NEUTRAL | No directional bias |
| 1 | LONG | Bullish bias |
| 2 | SHORT | Bearish bias |

---

**End of Specification**
