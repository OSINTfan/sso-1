# SSO-1 Architecture

**Version**: 1.2  
**Last Updated**: January 2026

---

## Overview

This document describes the end-to-end architecture of SSO-1, including data flow, component responsibilities, and trust boundaries. It serves as the primary reference for engineers integrating with or extending the protocol.

---

## 1. System Components

### 1.1 Component Inventory

| Component | Location | Language | Responsibility |
|-----------|----------|----------|----------------|
| Consumer | External | Any | Initiates signal requests, consumes signals |
| Switchboard Function | `oracle/offchain/function/` | Python | TEE execution container, signal computation |
| TEE Enclave | Runtime | N/A | Hardware-attested execution environment |
| On-Chain Program | `oracle/onchain/programs/sso_signal/` | Rust/Anchor | Verification, persistence, access control |
| Signal Account | On-Chain | N/A | Persistent storage for verified signals |

### 1.2 Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              External Consumers                                  │
│                     (Vaults, Trading Bots, Autonomous Agents)                   │
└─────────────────────────────────┬───────────────────────────────────────────────┘
                                  │
                                  │ (1) Pull Request / (6) Read Signal
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Solana Blockchain                                      │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │                        SSO Signal Program                                  │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐   │  │
│  │  │ SignalAccount   │  │  Instructions   │  │  Verification Logic     │   │  │
│  │  │ - MarketContext │  │ - initialize    │  │  - TEE attestation      │   │  │
│  │  │ - Assessment    │  │ - update_signal │  │  - Slot validation      │   │  │
│  │  │ - TeeReceipt    │  │                 │  │  - Authority checks     │   │  │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────┬───────────────────────────────────────────────┘
                                  │
                                  │ (5) Submit Transaction
                                  │
┌─────────────────────────────────┴───────────────────────────────────────────────┐
│                         Switchboard V3 Network                                   │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │                     On-Demand Function Executor                            │  │
│  │                                                                            │  │
│  │     (2) Dispatch ──────▶  ┌─────────────────────────────────────────┐     │  │
│  │                           │         TEE Enclave (AMD SEV-SNP)       │     │  │
│  │                           │  ┌─────────────────────────────────┐    │     │  │
│  │                           │  │     SSO-1 Function Container    │    │     │  │
│  │                           │  │  ┌───────────┐  ┌────────────┐  │    │     │  │
│  │     (4) Result + ◀─────── │  │  │  Scoring  │  │  Market    │  │    │     │  │
│  │         Attestation       │  │  │  Module   │  │  Context   │  │    │     │  │
│  │                           │  │  └───────────┘  └────────────┘  │    │     │  │
│  │                           │  └─────────────────────────────────┘    │     │  │
│  │                           │            │                            │     │  │
│  │                           │    (3) Generate Attestation             │     │  │
│  │                           └─────────────────────────────────────────┘     │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────┘
                                  │
                                  │ (External Data)
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                            Data Sources                                          │
│                   (Exchanges, DEXs, Market Data Providers)                      │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Data Flow

### 2.1 End-to-End Signal Update

```
Step  Actor              Action                                    Output
────  ─────              ──────                                    ──────
 1    Consumer           Initiates Switchboard Function call       Transaction
 2    Switchboard        Routes to TEE-enabled executor            Dispatch
 3    TEE Function       Fetches market data from sources          Raw data
 4    TEE Function       Derives MarketContext (objective)         MarketContext
 5    TEE Function       Computes SignalAssessment (subjective)    SignalAssessment
 6    TEE Enclave        Generates attestation over output         TeeReceipt
 7    Switchboard        Submits result transaction to Solana      Transaction
 8    On-Chain Program   Verifies TEE attestation                  Pass/Fail
 9    On-Chain Program   Validates slot constraints                Pass/Fail
10    On-Chain Program   Persists to SignalAccount                 Account update
11    Consumer           Reads verified signal from account        Signal data
```

### 2.2 Slot-Relative Validity Flow

```
                    valid_from_slot              valid_until_slot
                          │                            │
    ──────────────────────┼────────────────────────────┼──────────────────▶ Slot
                          │                            │
                          │◀────── VALID WINDOW ──────▶│
                          │                            │
    Signal Generated ─────┤                            │
                          │                            │
    Consumer Read ────────────────┬────────────────────│
                                  │                    │
                          VALID (use signal)           │
                                                       │
    Consumer Read ─────────────────────────────────────────┬────────────────
                                                           │
                                                   INVALID (reject)
```

### 2.3 Separation of Context and Assessment

```
┌─────────────────────────────────────────────────────────────────┐
│                        TEE Enclave                               │
│                                                                  │
│   ┌─────────────────────────┐    ┌─────────────────────────┐   │
│   │     MarketContext       │    │   SignalAssessment      │   │
│   │     (OBJECTIVE)         │───▶│   (SUBJECTIVE)          │   │
│   │                         │    │                         │   │
│   │  • Observable prices    │    │  • Direction inference  │   │
│   │  • Volume metrics       │    │  • Confidence scoring   │   │
│   │  • Liquidity depth      │    │  • Magnitude estimation │   │
│   │  • Volatility measures  │    │  • Validity window      │   │
│   │                         │    │                         │   │
│   │  Sources: Exchanges,    │    │  Model: Proprietary     │   │
│   │           DEXs, APIs    │    │         (protected)     │   │
│   └─────────────────────────┘    └─────────────────────────┘   │
│                                                                  │
│   HARD BOUNDARY: These MUST remain separate data structures     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Trust Boundaries

### 3.1 Trust Boundary Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  UNTRUSTED ZONE                                                             │
│                                                                             │
│    ┌───────────────┐         ┌───────────────────────────────────────┐     │
│    │   Consumer    │         │        Host Operating System          │     │
│    │   (External)  │         │        (Switchboard Operator)         │     │
│    └───────────────┘         └───────────────────────────────────────┘     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  TRUST BOUNDARY: TEE Attestation Verification                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TRUSTED ZONE (Hardware-Attested)                                          │
│                                                                             │
│    ┌───────────────────────────────────────────────────────────────────┐   │
│    │                    AMD SEV-SNP Enclave                             │   │
│    │                                                                    │   │
│    │    ┌────────────────────────────────────────────────────────┐     │   │
│    │    │              SSO-1 Function Container                   │     │   │
│    │    │                                                         │     │   │
│    │    │    • Model weights (confidential)                       │     │   │
│    │    │    • Scoring logic (integrity-protected)                │     │   │
│    │    │    • Attestation generation                             │     │   │
│    │    └────────────────────────────────────────────────────────┘     │   │
│    │                                                                    │   │
│    └───────────────────────────────────────────────────────────────────┘   │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  TRUST BOUNDARY: Solana Consensus                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TRUSTLESS ZONE (Blockchain-Verified)                                       │
│                                                                             │
│    ┌───────────────────────────────────────────────────────────────────┐   │
│    │                    SSO Signal On-Chain Program                     │   │
│    │                                                                    │   │
│    │    • TEE attestation verification                                  │   │
│    │    • Slot-relative validity enforcement                            │   │
│    │    • Signal persistence and access                                 │   │
│    └───────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Trust Assumptions

| Zone | Component | Trust Basis | Failure Mode |
|------|-----------|-------------|--------------|
| Untrusted | Consumer | None required | Byzantine behavior assumed |
| Untrusted | Host OS | None | Cannot access enclave memory |
| Trusted | TEE Enclave | AMD hardware attestation | Hardware compromise |
| Trustless | On-Chain | Solana consensus | Consensus attack |

### 3.3 Verification Chain

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  AMD Root    │────▶│  Platform    │────▶│  Enclave     │────▶│  Signal      │
│  of Trust    │     │  Attestation │     │  Measurement │     │  Output      │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │                    │
       │              Verified by          Compared to           Signed by
       │              AMD certs            known mr_enclave      enclave key
       ▼                    ▼                    ▼                    ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                         On-Chain Verification                                 │
│                                                                              │
│  1. Verify attestation signature chain                                       │
│  2. Check mr_enclave against allowlist                                       │
│  3. Validate signal signed by enclave key                                    │
│  4. Enforce slot constraints                                                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. On-Chain Program Architecture

### 4.1 Account Structure

```
┌─────────────────────────────────────────────────────────────────┐
│                      SignalAccount (PDA)                         │
│                                                                  │
│  Seeds: ["signal", asset_pair, authority]                       │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Header                                                    │  │
│  │  • version: u8                                             │  │
│  │  • authority: Pubkey                                       │  │
│  │  • asset_pair: [u8; 32]                                    │  │
│  │  • update_count: u64                                       │  │
│  │  • last_update_slot: u64                                   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  MarketContext (Embedded)                                  │  │
│  │  • slot: u64                                               │  │
│  │  • price: u64                                              │  │
│  │  • volume_24h: u64                                         │  │
│  │  • volatility_1h: u64                                      │  │
│  │  • liquidity_depth: u64                                    │  │
│  │  • source_bitmap: u64                                      │  │
│  │  • source_count: u8                                        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  SignalAssessment (Embedded)                               │  │
│  │  • signal_type: u8                                         │  │
│  │  • direction: u8                                           │  │
│  │  • magnitude: u8                                           │  │
│  │  • confidence: u8                                          │  │
│  │  • valid_from_slot: u64                                    │  │
│  │  • valid_until_slot: u64                                   │  │
│  │  • model_version: [u8; 8]                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  TeeReceipt (Embedded)                                     │  │
│  │  • enclave_signer: Pubkey                                  │  │
│  │  • attestation_hash: [u8; 32]                              │  │
│  │  • mr_enclave: [u8; 32]                                    │  │
│  │  • timestamp_slot: u64                                     │  │
│  │  • platform_version: u16                                   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Instruction Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     initialize_signal_account                    │
│                                                                  │
│  Inputs:                                                         │
│    • authority: Signer                                           │
│    • asset_pair: [u8; 32]                                        │
│                                                                  │
│  Validation:                                                     │
│    • Authority must sign                                         │
│    • Account must not exist                                      │
│                                                                  │
│  Output:                                                         │
│    • Initialized SignalAccount PDA                               │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                         update_signal                            │
│                                                                  │
│  Inputs:                                                         │
│    • signal_account: SignalAccount                               │
│    • market_context: MarketContext                               │
│    • signal_assessment: SignalAssessment                         │
│    • tee_receipt: TeeReceipt                                     │
│    • enclave_signer: Signer (TEE key)                            │
│                                                                  │
│  Validation:                                                     │
│    1. Verify enclave_signer matches tee_receipt.enclave_signer   │
│    2. Verify mr_enclave is in allowlist                          │
│    3. Verify current_slot <= valid_until_slot                    │
│    4. Verify valid_from_slot <= current_slot                     │
│                                                                  │
│  Effects:                                                        │
│    • Update market_context                                       │
│    • Update signal_assessment                                    │
│    • Update tee_receipt                                          │
│    • Increment update_count                                      │
│    • Set last_update_slot                                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. Off-Chain Function Architecture

### 5.1 Execution Environment

```
┌─────────────────────────────────────────────────────────────────┐
│                   Docker Container (TEE)                         │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Environment Variables                                     │  │
│  │  • SWITCHBOARD_FUNCTION_KEY                                │  │
│  │  • SWITCHBOARD_FUNCTION_REQUEST_KEY                        │  │
│  │  • DATA_SOURCE_URLS (comma-separated)                      │  │
│  │  • MODEL_CONFIG_PATH                                       │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  main.py                                                   │  │
│  │                                                            │  │
│  │  1. Parse Switchboard request                              │  │
│  │  2. Fetch market data from sources                         │  │
│  │  3. Derive MarketContext                                   │  │
│  │  4. Compute SignalAssessment                               │  │
│  │  5. Capture TEE attestation                                │  │
│  │  6. Return signed result to Switchboard                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Module: scoring/                                          │  │
│  │  • Signal computation logic                                │  │
│  │  • Model inference (if applicable)                         │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Module: tee/                                              │  │
│  │  • Attestation capture                                     │  │
│  │  • Platform verification                                   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Future Extensibility

### 6.1 Multi-Provider Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Future: Multi-Provider                       │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ Provider A  │  │ Provider B  │  │ Provider C  │              │
│  │ (TEE)       │  │ (TEE)       │  │ (TEE)       │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
│         └────────────────┼────────────────┘                      │
│                          ▼                                       │
│              ┌───────────────────────┐                          │
│              │   Aggregation Layer   │                          │
│              │   (On-Chain or TEE)   │                          │
│              └───────────────────────┘                          │
│                          │                                       │
│                          ▼                                       │
│              ┌───────────────────────┐                          │
│              │   Consensus Signal    │                          │
│              └───────────────────────┘                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

The current architecture explicitly supports future multi-provider expansion through:

1. **SignalAccount.source_bitmap**: Tracks contributing data sources
2. **TeeReceipt array support**: Account can be extended for multiple attestations
3. **Aggregation hooks**: Reserved instruction space for aggregation logic

---

## 7. Performance Characteristics

### 7.1 Latency Budget

| Phase | Target Latency | Notes |
|-------|----------------|-------|
| Consumer → Switchboard | < 50ms | Network dependent |
| Switchboard → TEE dispatch | < 100ms | Operator dependent |
| TEE execution | < 500ms | Model complexity dependent |
| Transaction submission | < 100ms | Network conditions |
| Finality | 100-150ms | Alpenglow era |
| **Total** | **< 1 second** | End-to-end |

### 7.2 Slot Considerations

- Solana slot time: ~400ms
- Recommended validity window: 5-25 slots (2-10 seconds)
- Minimum validity window: 3 slots (buffer for finality)

---

## 8. Security Invariants

1. **Slot Validity**: `current_slot <= valid_until_slot` MUST be enforced on-chain
2. **TEE Verification**: Only signals from verified enclaves are accepted
3. **Separation**: MarketContext and SignalAssessment MUST remain separate structures
4. **No Secrets On-Chain**: All on-chain data is public; secrets stay in TEE
5. **Pull-Only**: No push-based updates; consumers always initiate

---

**End of Architecture Document**
