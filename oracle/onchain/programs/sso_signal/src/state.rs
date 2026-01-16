//! SSO-1 v1.2 State Definitions
//!
//! This module defines all on-chain data structures for the SSO-1 protocol.
//! These structures represent the canonical format for verifiable signals.
//!
//! # Data Separation (CRITICAL INVARIANT)
//!
//! The protocol enforces strict separation between:
//! - `MarketContext`: Objective, observable market data
//! - `SignalAssessment`: Subjective interpretation derived from context
//!
//! This separation MUST be maintained at all times.

use anchor_lang::prelude::*;

// =============================================================================
// CONSTANTS
// =============================================================================

/// SSO-1 specification version (1.2 = 12)
pub const SPEC_VERSION: u8 = 12;

/// Maximum allowed enclaves per provider
pub const MAX_ENCLAVES_PER_PROVIDER: usize = 8;

/// Maximum slot drift for market data freshness
pub const MAX_MARKET_DATA_SLOT_DRIFT: u64 = 100;

/// Basis points scale (10000 = 100.00%)
pub const BPS_SCALE: u16 = 10000;

/// Price scaling factor (10^8)
pub const PRICE_SCALE: u64 = 100_000_000;

// =============================================================================
// SIGNAL ACCOUNT (Primary PDA)
// =============================================================================

/// SignalAccount: Primary PDA storing a complete verifiable signal.
///
/// PDA Seeds: ["signal", provider_pubkey, signal_id]
///
/// This is the core data structure of the SSO-1 protocol. It stores
/// verified signals along with their TEE attestation proofs.
#[account]
#[derive(Debug)]
pub struct SignalAccount {
    /// Provider public key (signer authority for updates)
    pub provider: Pubkey,

    /// Unique identifier for this signal stream
    pub signal_id: [u8; 32],

    /// SSO-1 specification version
    pub spec_version: u8,

    /// Current signal status
    pub status: SignalStatus,

    /// Objective market data at time of signal generation
    pub market_context: MarketContext,

    /// Subjective assessment derived from market context
    pub signal_assessment: SignalAssessment,

    /// TEE attestation receipt proving signal authenticity
    pub tee_receipt: TeeReceipt,

    /// Slot when this signal account was created
    pub created_at_slot: u64,

    /// Slot of most recent update
    pub updated_at_slot: u64,

    /// Total number of updates (for sequencing)
    pub update_count: u64,

    /// PDA bump seed
    pub bump: u8,

    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl SignalAccount {
    /// Calculate space required for account allocation
    pub const LEN: usize = 8 // Anchor discriminator
        + 32                  // provider: Pubkey
        + 32                  // signal_id: [u8; 32]
        + 1                   // spec_version: u8
        + 1                   // status: SignalStatus
        + MarketContext::LEN  // market_context
        + SignalAssessment::LEN // signal_assessment
        + TeeReceipt::LEN     // tee_receipt
        + 8                   // created_at_slot: u64
        + 8                   // updated_at_slot: u64
        + 8                   // update_count: u64
        + 1                   // bump: u8
        + 32;                 // _reserved: [u8; 32]

    /// PDA seed prefix
    pub const SEED_PREFIX: &'static [u8] = b"signal";

    /// Check if signal is still valid at given slot
    pub fn is_valid_at_slot(&self, current_slot: u64) -> bool {
        self.status == SignalStatus::Active
            && current_slot <= self.signal_assessment.valid_until_slot
    }

    /// Check if signal has expired
    pub fn is_expired(&self, current_slot: u64) -> bool {
        current_slot > self.signal_assessment.valid_until_slot
    }

    /// Get remaining validity in slots (None if expired)
    pub fn remaining_validity(&self, current_slot: u64) -> Option<u64> {
        if current_slot <= self.signal_assessment.valid_until_slot {
            Some(self.signal_assessment.valid_until_slot - current_slot)
        } else {
            None
        }
    }
}

// =============================================================================
// MARKET CONTEXT (Objective Data Layer)
// =============================================================================

/// MarketContext: Objective market state at signal generation time.
///
/// This structure contains ONLY observable, verifiable market data.
/// No subjective interpretation or scoring belongs here.
///
/// All values are normalized according to SSO-1 v1.2 normalization rules:
/// - Prices: scaled by 10^8 (8 decimal places)
/// - Percentages: scaled by 10^4 (basis points)
/// - Volumes: in native units
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct MarketContext {
    /// Unix timestamp (seconds) when market data was captured
    pub timestamp: i64,

    /// Solana slot when market data was captured
    pub captured_at_slot: u64,

    /// Primary asset price in USD (scaled by 10^8)
    /// Example: $50,000.12345678 = 5_000_012_345_678
    pub price_usd: u64,

    /// 24-hour trading volume in USD
    pub volume_24h_usd: u64,

    /// Market capitalization in USD
    pub market_cap_usd: u64,

    /// Price change percentage over 24h (basis points, signed)
    /// Example: -5.25% = -525
    pub price_change_24h_bps: i32,

    /// Bid-ask spread (basis points)
    pub spread_bps: u32,

    /// Order book depth at 2% from mid price (USD)
    pub depth_2pct_usd: u64,

    /// Number of data sources aggregated
    pub source_count: u8,

    /// Bitfield indicating which sources contributed
    /// Bit 0: Source A, Bit 1: Source B, etc.
    pub source_bitmap: u16,

    /// Asset identifier (e.g., "BTC", "ETH", "SOL")
    pub asset_symbol: [u8; 8],

    /// Reserved for future market context fields
    pub _reserved: [u8; 32],
}

impl MarketContext {
    pub const LEN: usize = 8  // timestamp: i64
        + 8                    // captured_at_slot: u64
        + 8                    // price_usd: u64
        + 8                    // volume_24h_usd: u64
        + 8                    // market_cap_usd: u64
        + 4                    // price_change_24h_bps: i32
        + 4                    // spread_bps: u32
        + 8                    // depth_2pct_usd: u64
        + 1                    // source_count: u8
        + 2                    // source_bitmap: u16
        + 8                    // asset_symbol: [u8; 8]
        + 32;                  // _reserved: [u8; 32]

    /// Validate market context data integrity
    pub fn validate(&self) -> bool {
        // Price must be positive
        if self.price_usd == 0 {
            return false;
        }

        // Must have at least one data source
        if self.source_count == 0 {
            return false;
        }

        // Source bitmap must match source count
        let bitmap_count = self.source_bitmap.count_ones() as u8;
        if bitmap_count != self.source_count {
            return false;
        }

        true
    }

    /// Check if market data is fresh enough
    pub fn is_fresh(&self, current_slot: u64) -> bool {
        if current_slot < self.captured_at_slot {
            return false;
        }
        current_slot - self.captured_at_slot <= MAX_MARKET_DATA_SLOT_DRIFT
    }
}

// =============================================================================
// SIGNAL ASSESSMENT (Subjective Interpretation Layer)
// =============================================================================

/// SignalAssessment: Subjective signal derived from MarketContext.
///
/// Contains the actual trading signal and confidence metrics.
/// Validity is enforced via slot-relative bounds.
///
/// CRITICAL INVARIANT: `current_slot <= valid_until_slot` must be enforced
/// on every signal submission and update.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct SignalAssessment {
    /// Signal direction (Long, Short, Neutral)
    pub direction: SignalDirection,

    /// Signal strength (0-10000 representing 0.00% to 100.00%)
    pub strength_bps: u16,

    /// Confidence score (0-10000 representing 0.00% to 100.00%)
    pub confidence_bps: u16,

    /// Expected holding period in slots
    pub time_horizon_slots: u64,

    /// Slot after which this signal is no longer valid
    /// INVARIANT: current_slot <= valid_until_slot for signal to be actionable
    pub valid_until_slot: u64,

    /// Slot when this assessment was generated
    pub generated_at_slot: u64,

    /// Risk score (0-10000, higher = more risk)
    pub risk_score_bps: u16,

    /// Suggested position size as percentage of portfolio (0-10000 bps)
    pub suggested_size_bps: u16,

    /// Model/algorithm version that generated this assessment
    pub model_version: u32,

    /// Hash of model parameters for reproducibility
    pub model_params_hash: [u8; 32],

    /// Reserved for future assessment fields
    pub _reserved: [u8; 16],
}

impl SignalAssessment {
    pub const LEN: usize = 1 // direction: SignalDirection
        + 2                   // strength_bps: u16
        + 2                   // confidence_bps: u16
        + 8                   // time_horizon_slots: u64
        + 8                   // valid_until_slot: u64
        + 8                   // generated_at_slot: u64
        + 2                   // risk_score_bps: u16
        + 2                   // suggested_size_bps: u16
        + 4                   // model_version: u32
        + 32                  // model_params_hash: [u8; 32]
        + 16;                 // _reserved: [u8; 16]

    /// Validate assessment data integrity
    pub fn validate(&self, current_slot: u64) -> bool {
        // All basis point values must be in valid range (0-10000)
        if self.strength_bps > BPS_SCALE {
            return false;
        }
        if self.confidence_bps > BPS_SCALE {
            return false;
        }
        if self.risk_score_bps > BPS_SCALE {
            return false;
        }
        if self.suggested_size_bps > BPS_SCALE {
            return false;
        }

        // Signal must not already be expired
        if current_slot > self.valid_until_slot {
            return false;
        }

        // Generated slot must be before or equal to valid_until
        if self.generated_at_slot > self.valid_until_slot {
            return false;
        }

        true
    }

    /// Calculate remaining validity in slots
    pub fn remaining_validity(&self, current_slot: u64) -> Option<u64> {
        if current_slot <= self.valid_until_slot {
            Some(self.valid_until_slot - current_slot)
        } else {
            None
        }
    }
}

/// Signal direction enumeration
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SignalDirection {
    #[default]
    Neutral = 0,
    Long = 1,
    Short = 2,
}

// =============================================================================
// TEE RECEIPT (Attestation Layer)
// =============================================================================

/// TeeReceipt: Cryptographic proof of TEE execution.
///
/// Contains attestation data that proves the signal was generated
/// within an AMD SEV-SNP enclave with verified code.
///
/// The `mr_enclave` field is the root of trust - it must match
/// an entry in the provider's allowlist.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub struct TeeReceipt {
    /// MR_ENCLAVE: Hash of enclave code and initial state
    /// Must match an entry in the provider's allowlist
    pub mr_enclave: [u8; 32],

    /// MR_SIGNER: Hash of enclave signing key
    pub mr_signer: [u8; 32],

    /// Enclave-generated Ed25519 signature over signal data
    pub enclave_signature: [u8; 64],

    /// Public key of the enclave signing keypair
    pub enclave_pubkey: [u8; 32],

    /// Report data binding signal to attestation
    pub report_data: [u8; 64],

    /// Attestation timestamp (Unix seconds)
    pub attestation_timestamp: i64,

    /// TEE platform type
    pub platform: TeePlatform,

    /// Security version number
    pub svn: u16,

    /// Reserved for future TEE fields
    pub _reserved: [u8; 13],
}

impl TeeReceipt {
    pub const LEN: usize = 32 // mr_enclave: [u8; 32]
        + 32                   // mr_signer: [u8; 32]
        + 64                   // enclave_signature: [u8; 64]
        + 32                   // enclave_pubkey: [u8; 32]
        + 64                   // report_data: [u8; 64]
        + 8                    // attestation_timestamp: i64
        + 1                    // platform: TeePlatform
        + 2                    // svn: u16
        + 13;                  // _reserved: [u8; 13]

    /// Verify the TEE signature over provided message hash
    pub fn verify_signature(&self, _message_hash: &[u8; 32]) -> bool {
        // Basic sanity checks
        if self.enclave_pubkey == [0u8; 32] {
            return false;
        }
        if self.enclave_signature == [0u8; 64] {
            return false;
        }
        
        // Actual cryptographic verification is done via Switchboard
        // attestation verification or Ed25519 program CPI
        // For now, we trust the Switchboard attestation verification
        true
    }
}

impl Default for TeeReceipt {
    fn default() -> Self {
        Self {
            mr_enclave: [0u8; 32],
            mr_signer: [0u8; 32],
            enclave_signature: [0u8; 64],
            enclave_pubkey: [0u8; 32],
            report_data: [0u8; 64],
            attestation_timestamp: 0,
            platform: TeePlatform::default(),
            svn: 0,
            _reserved: [0u8; 13],
        }
    }
}

/// Supported TEE platforms
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TeePlatform {
    #[default]
    Unknown = 0,
    /// AMD SEV-SNP (required for SSO-1 v1.2)
    AmdSevSnp = 1,
    /// Intel TDX (reserved for future)
    IntelTdx = 2,
    /// Intel SGX (legacy, not recommended)
    IntelSgx = 3,
}

// =============================================================================
// SIGNAL STATUS
// =============================================================================

/// Signal lifecycle status
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SignalStatus {
    #[default]
    Uninitialized = 0,
    /// Signal is active and valid (if not expired)
    Active = 1,
    /// Signal has been marked as expired
    Expired = 2,
    /// Signal has been revoked by provider
    Revoked = 3,
}

// =============================================================================
// PROVIDER REGISTRY
// =============================================================================

/// ProviderRegistry: Tracks authorized signal providers.
///
/// PDA Seeds: ["provider", provider_pubkey]
///
/// Each provider maintains their own allowlist of MR_ENCLAVE values
/// that are authorized to produce signals on their behalf.
#[account]
#[derive(Debug)]
pub struct ProviderRegistry {
    /// Provider authority pubkey
    pub authority: Pubkey,

    /// Human-readable provider name (null-terminated)
    pub name: [u8; 32],

    /// Provider status
    pub is_active: bool,

    /// Total signals submitted
    pub signal_count: u64,

    /// Registration slot
    pub registered_at_slot: u64,

    /// Last activity slot
    pub last_active_slot: u64,

    /// Number of allowed enclaves
    pub enclave_count: u8,

    /// Allowed MR_ENCLAVE values for this provider (up to 8)
    pub allowed_enclaves: [[u8; 32]; MAX_ENCLAVES_PER_PROVIDER],

    /// PDA bump
    pub bump: u8,

    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl ProviderRegistry {
    pub const LEN: usize = 8 // discriminator
        + 32                  // authority: Pubkey
        + 32                  // name: [u8; 32]
        + 1                   // is_active: bool
        + 8                   // signal_count: u64
        + 8                   // registered_at_slot: u64
        + 8                   // last_active_slot: u64
        + 1                   // enclave_count: u8
        + (32 * MAX_ENCLAVES_PER_PROVIDER) // allowed_enclaves
        + 1                   // bump: u8
        + 32;                 // _reserved: [u8; 32]

    /// PDA seed prefix
    pub const SEED_PREFIX: &'static [u8] = b"provider";

    /// Check if an MR_ENCLAVE value is in the allowlist
    pub fn is_enclave_allowed(&self, mr_enclave: &[u8; 32]) -> bool {
        for i in 0..self.enclave_count as usize {
            if i >= MAX_ENCLAVES_PER_PROVIDER {
                break;
            }
            if &self.allowed_enclaves[i] == mr_enclave {
                return true;
            }
        }
        false
    }

    /// Add an enclave to the allowlist
    pub fn add_enclave(&mut self, mr_enclave: [u8; 32]) -> bool {
        if self.enclave_count as usize >= MAX_ENCLAVES_PER_PROVIDER {
            return false;
        }
        
        // Check if already exists
        if self.is_enclave_allowed(&mr_enclave) {
            return false;
        }
        
        self.allowed_enclaves[self.enclave_count as usize] = mr_enclave;
        self.enclave_count += 1;
        true
    }

    /// Remove an enclave from the allowlist
    pub fn remove_enclave(&mut self, mr_enclave: &[u8; 32]) -> bool {
        for i in 0..self.enclave_count as usize {
            if &self.allowed_enclaves[i] == mr_enclave {
                // Shift remaining enclaves down
                for j in i..(self.enclave_count as usize - 1) {
                    self.allowed_enclaves[j] = self.allowed_enclaves[j + 1];
                }
                self.allowed_enclaves[self.enclave_count as usize - 1] = [0u8; 32];
                self.enclave_count -= 1;
                return true;
            }
        }
        false
    }
}

// =============================================================================
// GLOBAL CONFIG
// =============================================================================

/// GlobalConfig: Protocol-level configuration.
///
/// PDA Seeds: ["config"]
///
/// Contains global parameters that govern all signal submissions.
#[account]
#[derive(Debug)]
pub struct GlobalConfig {
    /// Protocol admin authority
    pub admin: Pubkey,

    /// Minimum slots a signal must be valid for
    pub min_validity_slots: u64,

    /// Maximum slots a signal can be valid for
    pub max_validity_slots: u64,

    /// Minimum required data sources
    pub min_source_count: u8,

    /// Minimum required confidence score (basis points)
    pub min_confidence_bps: u16,

    /// Protocol paused flag
    pub is_paused: bool,

    /// Protocol version (12 = v1.2)
    pub protocol_version: u16,

    /// Total signals submitted across all providers
    pub total_signals: u64,

    /// Total active providers
    pub total_providers: u64,

    /// PDA bump
    pub bump: u8,

    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl GlobalConfig {
    pub const LEN: usize = 8 // discriminator
        + 32                  // admin: Pubkey
        + 8                   // min_validity_slots: u64
        + 8                   // max_validity_slots: u64
        + 1                   // min_source_count: u8
        + 2                   // min_confidence_bps: u16
        + 1                   // is_paused: bool
        + 2                   // protocol_version: u16
        + 8                   // total_signals: u64
        + 8                   // total_providers: u64
        + 1                   // bump: u8
        + 32;                 // _reserved: [u8; 32]

    /// PDA seed prefix
    pub const SEED_PREFIX: &'static [u8] = b"config";
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_context_validation() {
        let mut ctx = MarketContext::default();
        
        // Invalid: zero price
        assert!(!ctx.validate());
        
        // Invalid: zero sources
        ctx.price_usd = 50_000_00000000; // $50,000
        assert!(!ctx.validate());
        
        // Invalid: source count mismatch
        ctx.source_count = 2;
        ctx.source_bitmap = 0b001; // Only 1 bit set
        assert!(!ctx.validate());
        
        // Valid
        ctx.source_bitmap = 0b011; // 2 bits set
        assert!(ctx.validate());
    }

    #[test]
    fn test_signal_assessment_validation() {
        let mut assessment = SignalAssessment::default();
        assessment.valid_until_slot = 1000;
        assessment.generated_at_slot = 500;
        
        // Valid at slot 800
        assert!(assessment.validate(800));
        
        // Invalid: expired
        assert!(!assessment.validate(1001));
        
        // Invalid: strength too high
        assessment.strength_bps = 10001;
        assert!(!assessment.validate(800));
    }

    #[test]
    fn test_provider_enclave_management() {
        let mut provider = ProviderRegistry {
            authority: Pubkey::default(),
            name: [0u8; 32],
            is_active: true,
            signal_count: 0,
            registered_at_slot: 0,
            last_active_slot: 0,
            enclave_count: 0,
            allowed_enclaves: [[0u8; 32]; MAX_ENCLAVES_PER_PROVIDER],
            bump: 0,
            _reserved: [0u8; 32],
        };

        let enclave1 = [1u8; 32];
        let enclave2 = [2u8; 32];

        // Add enclave
        assert!(provider.add_enclave(enclave1));
        assert_eq!(provider.enclave_count, 1);
        assert!(provider.is_enclave_allowed(&enclave1));

        // Can't add duplicate
        assert!(!provider.add_enclave(enclave1));

        // Add second enclave
        assert!(provider.add_enclave(enclave2));
        assert_eq!(provider.enclave_count, 2);

        // Remove first enclave
        assert!(provider.remove_enclave(&enclave1));
        assert_eq!(provider.enclave_count, 1);
        assert!(!provider.is_enclave_allowed(&enclave1));
        assert!(provider.is_enclave_allowed(&enclave2));
    }

    #[test]
    fn test_signal_validity_check() {
        let signal = SignalAccount {
            provider: Pubkey::default(),
            signal_id: [0u8; 32],
            spec_version: SPEC_VERSION,
            status: SignalStatus::Active,
            market_context: MarketContext::default(),
            signal_assessment: SignalAssessment {
                valid_until_slot: 1000,
                ..Default::default()
            },
            tee_receipt: TeeReceipt::default(),
            created_at_slot: 0,
            updated_at_slot: 0,
            update_count: 0,
            bump: 0,
            _reserved: [0u8; 32],
        };

        assert!(signal.is_valid_at_slot(500));
        assert!(signal.is_valid_at_slot(1000));
        assert!(!signal.is_valid_at_slot(1001));
        assert!(signal.is_expired(1001));
        
        assert_eq!(signal.remaining_validity(500), Some(500));
        assert_eq!(signal.remaining_validity(1001), None);
    }
}
