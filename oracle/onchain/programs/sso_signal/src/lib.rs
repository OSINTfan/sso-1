//! SSO-1: Standardized Verifiable Signal Oracle
//!
//! An open, verifiable protocol for standardized signal formatting
//! on Solana with Trusted Execution Environment (TEE) attestation.
//!
//! ## Overview
//!
//! SSO-1 provides a canonical on-chain format for verifiable signals,
//! backed by AMD SEV-SNP attestation proofs via Switchboard V3 On-Demand.
//!
//! ## Key Features
//!
//! - **Verifiable Signals**: Every signal includes TEE attestation proof
//! - **Slot-Relative Validity**: Signals expire based on Solana slots
//! - **Data Separation**: Clear boundary between objective MarketContext
//!   and subjective SignalAssessment
//! - **Provider Registry**: Managed allowlist of authorized TEE enclaves
//!
//! ## Specification Version
//!
//! This implementation follows SSO-1 v1.2 specification.
//!
//! ## Safety Considerations
//!
//! - All state modifications are atomic within transactions
//! - Slot-based validity provides deterministic expiration
//! - TEE attestation ensures signal authenticity

use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;
use state::*;

declare_id!("SSo1111111111111111111111111111111111111111");

/// SSO-1 Program
#[program]
pub mod sso_signal {
    use super::*;

    // =========================================================================
    // CONFIGURATION INSTRUCTIONS
    // =========================================================================

    /// Initialize the global protocol configuration.
    ///
    /// This must be called once before any other protocol operations.
    /// Only the deployer/admin can call this.
    ///
    /// # Arguments
    ///
    /// * `min_validity_slots` - Minimum slots a signal must be valid for
    /// * `max_validity_slots` - Maximum slots a signal can be valid for
    /// * `min_source_count` - Minimum required data sources in MarketContext
    /// * `min_confidence_bps` - Minimum confidence score (basis points)
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        min_validity_slots: u64,
        max_validity_slots: u64,
        min_source_count: u8,
        min_confidence_bps: u16,
    ) -> Result<()> {
        handle_initialize_config(
            ctx,
            min_validity_slots,
            max_validity_slots,
            min_source_count,
            min_confidence_bps,
        )
    }

    /// Update protocol configuration parameters.
    ///
    /// Admin-only instruction to modify protocol parameters.
    ///
    /// # Arguments
    ///
    /// * `min_validity_slots` - New minimum validity (None to keep current)
    /// * `max_validity_slots` - New maximum validity (None to keep current)
    /// * `min_source_count` - New minimum sources (None to keep current)
    /// * `min_confidence_bps` - New minimum confidence (None to keep current)
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        min_validity_slots: Option<u64>,
        max_validity_slots: Option<u64>,
        min_source_count: Option<u8>,
        min_confidence_bps: Option<u16>,
    ) -> Result<()> {
        handle_update_config(
            ctx,
            min_validity_slots,
            max_validity_slots,
            min_source_count,
            min_confidence_bps,
        )
    }

    /// Pause or unpause the protocol.
    ///
    /// Admin-only instruction. When paused, no new signals can be submitted.
    ///
    /// # Arguments
    ///
    /// * `paused` - True to pause, false to unpause
    pub fn pause_protocol(ctx: Context<PauseProtocol>, paused: bool) -> Result<()> {
        handle_pause_protocol(ctx, paused)
    }

    // =========================================================================
    // PROVIDER MANAGEMENT INSTRUCTIONS
    // =========================================================================

    /// Register a new signal provider.
    ///
    /// Creates a ProviderRegistry PDA for the caller.
    /// Optionally initializes with one enclave in the allowlist.
    ///
    /// # Arguments
    ///
    /// * `name` - Provider name (32 bytes, null-terminated)
    /// * `initial_enclave` - Optional MR_ENCLAVE to add to allowlist
    pub fn register_provider(
        ctx: Context<RegisterProvider>,
        name: [u8; 32],
        initial_enclave: Option<[u8; 32]>,
    ) -> Result<()> {
        handle_register_provider(ctx, name, initial_enclave)
    }

    /// Deactivate a provider.
    ///
    /// Can be called by the provider authority or protocol admin.
    /// Deactivated providers cannot submit new signals.
    pub fn deactivate_provider(ctx: Context<DeactivateProvider>) -> Result<()> {
        handle_deactivate_provider(ctx)
    }

    /// Add an enclave to provider's allowlist.
    ///
    /// Provider-only instruction. Maximum 8 enclaves per provider.
    ///
    /// # Arguments
    ///
    /// * `mr_enclave` - MR_ENCLAVE hash to add (32 bytes)
    pub fn add_enclave(ctx: Context<AddEnclave>, mr_enclave: [u8; 32]) -> Result<()> {
        handle_add_enclave(ctx, mr_enclave)
    }

    /// Remove an enclave from provider's allowlist.
    ///
    /// Provider-only instruction.
    ///
    /// # Arguments
    ///
    /// * `mr_enclave` - MR_ENCLAVE hash to remove (32 bytes)
    pub fn remove_enclave(ctx: Context<RemoveEnclave>, mr_enclave: [u8; 32]) -> Result<()> {
        handle_remove_enclave(ctx, mr_enclave)
    }

    // =========================================================================
    // SIGNAL OPERATION INSTRUCTIONS
    // =========================================================================

    /// Submit a new verified signal.
    ///
    /// Creates a SignalAccount PDA with the provided data.
    /// Validates TEE attestation and enforces slot-relative validity.
    ///
    /// # Arguments
    ///
    /// * `signal_id` - Unique identifier for this signal stream (32 bytes)
    /// * `market_context` - Objective market data at signal generation
    /// * `signal_assessment` - Subjective signal interpretation
    /// * `tee_receipt` - TEE attestation proof
    ///
    /// # Errors
    ///
    /// * `ProtocolPaused` - Protocol is currently paused
    /// * `InvalidMarketContext` - Market data validation failed
    /// * `InvalidSignalAssessment` - Signal assessment validation failed
    /// * `EnclaveNotAllowed` - TEE enclave not in provider's allowlist
    /// * `SignalExpired` - Signal validity already expired
    pub fn submit_signal(
        ctx: Context<SubmitSignal>,
        signal_id: [u8; 32],
        market_context: MarketContext,
        signal_assessment: SignalAssessment,
        tee_receipt: TeeReceipt,
    ) -> Result<()> {
        handle_submit_signal(ctx, signal_id, market_context, signal_assessment, tee_receipt)
    }

    /// Update an existing signal.
    ///
    /// Updates the market context and signal assessment for a signal.
    /// Signal must be active and not expired.
    ///
    /// # Arguments
    ///
    /// * `market_context` - New market context
    /// * `signal_assessment` - New signal assessment
    /// * `tee_receipt` - New TEE attestation proof
    ///
    /// # Errors
    ///
    /// * `InvalidSignalState` - Signal is not active
    /// * `SignalExpired` - Signal has expired
    /// * `EnclaveNotAllowed` - TEE enclave not in allowlist
    pub fn update_signal(
        ctx: Context<UpdateSignal>,
        market_context: MarketContext,
        signal_assessment: SignalAssessment,
        tee_receipt: TeeReceipt,
    ) -> Result<()> {
        handle_update_signal(ctx, market_context, signal_assessment, tee_receipt)
    }

    /// Revoke an existing signal.
    ///
    /// Marks the signal as revoked. Cannot be undone.
    /// Provider-only instruction.
    ///
    /// # Errors
    ///
    /// * `InvalidSignalState` - Signal is not active
    /// * `ProviderAuthorityMismatch` - Caller is not the signal provider
    pub fn revoke_signal(ctx: Context<RevokeSignal>) -> Result<()> {
        handle_revoke_signal(ctx)
    }
}

// =============================================================================
// PROGRAM ID HELPERS
// =============================================================================

/// Derives the GlobalConfig PDA address
pub fn derive_config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[GlobalConfig::SEED_PREFIX], &crate::ID)
}

/// Derives a ProviderRegistry PDA address
pub fn derive_provider_address(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ProviderRegistry::SEED_PREFIX, authority.as_ref()],
        &crate::ID,
    )
}

/// Derives a SignalAccount PDA address
pub fn derive_signal_address(provider: &Pubkey, signal_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SignalAccount::SEED_PREFIX, provider.as_ref(), signal_id.as_ref()],
        &crate::ID,
    )
}
