# SSO-1 Signal Schema Documentation

**Specification Version**: 1.2  
**Last Updated**: January 2026

---

## Overview

This document provides human-readable documentation for all SSO-1 data structures. It serves as the authoritative reference for implementers and consumers of the protocol.

---

## 1. MarketContext

**Purpose**: Objective, observable market data at a specific slot.

**Constraints**: 
- MUST contain only observable data
- MUST NOT contain subjective assessments
- MUST include the slot of observation

### Fields

| Field | Type | Size | Description | Units/Scaling |
|-------|------|------|-------------|---------------|
| `slot` | u64 | 8 bytes | Solana slot at observation | Slot number |
| `price` | u64 | 8 bytes | Current price | Base units × 10^9 |
| `volume_24h` | u64 | 8 bytes | 24-hour trading volume | Quote units |
| `volatility_1h` | u64 | 8 bytes | 1-hour realized volatility | Percentage × 10^6 |
| `liquidity_depth` | u64 | 8 bytes | Available liquidity within 2% of mid | Quote units |
| `source_bitmap` | u64 | 8 bytes | Bitmap of contributing sources | Bitfield |
| `source_count` | u8 | 1 byte | Number of sources aggregated | Count |
| `_reserved` | [u8; 7] | 7 bytes | Reserved for future use | N/A |

**Total Size**: 56 bytes

### Field Details

#### `slot`
The Solana slot at which the market data was observed. This provides the temporal reference for all other fields.

#### `price`
The current mid-market price, scaled by 10^9 for precision.

**Example**:
- Actual price: 100.50 USDC
- Stored value: 100_500_000_000

#### `volume_24h`
The total trading volume over the past 24 hours in quote currency units.

#### `volatility_1h`
One-hour realized volatility, calculated as the standard deviation of log returns, annualized, and scaled by 10^6.

**Example**:
- Actual volatility: 45.5% annualized
- Stored value: 455_000

#### `liquidity_depth`
The total available liquidity (both bid and ask) within 2% of the current mid-market price.

#### `source_bitmap`
A 64-bit bitmap indicating which data sources contributed to this observation.

**Bit Assignments** (TODO: Define official assignments):
| Bit | Source |
|-----|--------|
| 0 | Reserved |
| 1-63 | Provider-defined |

#### `source_count`
The number of data sources that contributed to this observation. Used for confidence calculation.

---

## 2. SignalAssessment

**Purpose**: Subjective interpretation derived from MarketContext.

**Constraints**:
- MUST be derived from MarketContext
- MUST be generated within TEE
- MUST specify valid_until_slot

### Fields

| Field | Type | Size | Description | Range |
|-------|------|------|-------------|-------|
| `signal_type` | SignalType | 1 byte | Category of signal | 0-3 (see enum) |
| `direction` | Direction | 1 byte | Directional bias | 0-2 (see enum) |
| `magnitude` | u8 | 1 byte | Signal strength | 0-100 |
| `confidence` | u8 | 1 byte | Confidence score | 0-100 |
| `_padding` | [u8; 4] | 4 bytes | Alignment padding | N/A |
| `valid_from_slot` | u64 | 8 bytes | First valid slot | Slot number |
| `valid_until_slot` | u64 | 8 bytes | Last valid slot | Slot number |
| `model_version` | [u8; 8] | 8 bytes | Model identifier | Hex string |

**Total Size**: 32 bytes

### Field Details

#### `signal_type`
Enumerated signal category:

| Value | Name | Description |
|-------|------|-------------|
| 0 | MOMENTUM | Trend-following signal |
| 1 | MEAN_REVERSION | Counter-trend signal |
| 2 | VOLATILITY | Volatility regime signal |
| 3 | LIQUIDITY | Liquidity condition signal |
| 4-255 | RESERVED | Reserved for future use |

#### `direction`
Directional bias of the signal:

| Value | Name | Description |
|-------|------|-------------|
| 0 | NEUTRAL | No directional bias |
| 1 | LONG | Bullish / buy bias |
| 2 | SHORT | Bearish / sell bias |

#### `magnitude`
Signal strength on a normalized 0-100 scale.

**Interpretation**:
- 0-20: Very weak signal
- 21-40: Weak signal
- 41-60: Moderate signal
- 61-80: Strong signal
- 81-100: Very strong signal

#### `confidence`
Confidence in the signal, 0-100 scale.

**Factors affecting confidence**:
- Number of data sources (more = higher)
- Data freshness (fresher = higher)
- Liquidity depth (higher = higher)
- Volatility regime (lower vol = higher)

#### `valid_from_slot` / `valid_until_slot`
Defines the validity window for the signal.

**Critical Invariant**: 
On-chain programs MUST enforce `current_slot <= valid_until_slot` before accepting the signal.

**Recommended Windows**:
- High-frequency: 3-5 slots (1.2-2 seconds)
- Standard: 10-25 slots (4-10 seconds)
- Low-frequency: 50-100 slots (20-40 seconds)

#### `model_version`
8-byte identifier for the model/logic that produced the signal.

**Format**: First 4 bytes = major.minor, last 4 bytes = build hash

**Example**: `01020001DEADBEEF` = v1.2.1, build DEADBEEF

---

## 3. TeeReceipt

**Purpose**: Cryptographic attestation from the TEE enclave.

**Constraints**:
- MUST be generated by AMD SEV-SNP (v1.2)
- MUST contain verifiable enclave measurement
- MUST be validated on-chain

### Fields

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `enclave_signer` | Pubkey | 32 bytes | TEE enclave public key |
| `attestation_hash` | [u8; 32] | 32 bytes | SHA-256 of attestation document |
| `mr_enclave` | [u8; 32] | 32 bytes | Enclave measurement |
| `timestamp_slot` | u64 | 8 bytes | Slot at attestation |
| `platform_version` | u16 | 2 bytes | TEE platform version |
| `_reserved` | [u8; 6] | 6 bytes | Reserved for future use |

**Total Size**: 112 bytes

### Field Details

#### `enclave_signer`
The Ed25519 public key of the TEE enclave. This key is generated inside the enclave and bound to the attestation.

**Verification**: The on-chain program verifies that transactions are signed by this key.

#### `attestation_hash`
SHA-256 hash of the full attestation report. Used for audit trails and verification.

#### `mr_enclave`
The measurement register containing a hash of the enclave code and initial state. This is the root of trust for code integrity.

**Verification**: Must match values in a configured allowlist.

#### `timestamp_slot`
The Solana slot at which the attestation was generated.

#### `platform_version`
Version of the TEE platform firmware. Used for compatibility and security checks.

---

## 4. SignalAccount

**Purpose**: On-chain account storing verified signals.

**Seeds**: `["signal", asset_pair, authority]`

### Fields

| Field | Type | Size | Description |
|-------|------|------|-------------|
| (discriminator) | [u8; 8] | 8 bytes | Anchor discriminator |
| `version` | u8 | 1 byte | Account version |
| `bump` | u8 | 1 byte | PDA bump seed |
| `_padding` | [u8; 6] | 6 bytes | Alignment padding |
| `authority` | Pubkey | 32 bytes | Update authority |
| `asset_pair` | [u8; 32] | 32 bytes | Asset pair identifier |
| `market_context` | MarketContext | 56 bytes | Latest market data |
| `signal_assessment` | SignalAssessment | 32 bytes | Latest signal |
| `tee_receipt` | TeeReceipt | 112 bytes | Latest attestation |
| `update_count` | u64 | 8 bytes | Total updates |
| `last_update_slot` | u64 | 8 bytes | Last update slot |

**Total Size**: 296 bytes (+ 8 byte discriminator = 304 bytes)

---

## 5. Normalization Rules

### Asset Pair Encoding

Asset pairs are encoded as 32-byte identifiers:

1. Format as `BASE/QUOTE` (e.g., `SOL/USDC`)
2. Encode as UTF-8
3. Pad with null bytes to 32 bytes

**Example**:
```
"SOL/USDC" -> [83, 79, 76, 47, 85, 83, 68, 67, 0, 0, ...] (32 bytes)
```

### Price Scaling

All prices use 10^9 scaling (9 decimal places):

```
actual_price = stored_price / 10^9
```

### Volatility Scaling

Volatility uses 10^6 scaling (6 decimal places, percentage):

```
actual_volatility = stored_volatility / 10^6
```

---

## 6. Versioning

### Schema Version

This document describes schema version **1.2**.

### Compatibility

| Version | Compatible With | Notes |
|---------|-----------------|-------|
| 1.2 | 1.1, 1.0 | Backward compatible |
| 1.1 | 1.0 | Added source_bitmap |
| 1.0 | N/A | Initial version |

### Migration

Account migrations are performed via explicit upgrade instructions. The `version` field in SignalAccount tracks the current schema version.

---

## 7. Serialization

### On-Chain (Anchor)

All structures use Anchor's `AnchorSerialize` / `AnchorDeserialize` traits with Borsh encoding.

### Off-Chain (Python)

Python uses `struct` module for binary serialization matching Borsh layout.

### Wire Format

```
MarketContext:
  [0:8]   slot (u64 LE)
  [8:16]  price (u64 LE)
  [16:24] volume_24h (u64 LE)
  [24:32] volatility_1h (u64 LE)
  [32:40] liquidity_depth (u64 LE)
  [40:48] source_bitmap (u64 LE)
  [48:49] source_count (u8)
  [49:56] _reserved (7 bytes)
```

---

**End of Schema Documentation**
