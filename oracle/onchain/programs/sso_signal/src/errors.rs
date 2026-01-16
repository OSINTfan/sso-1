//! SSO-1 v1.2 Error Definitions
//!
//! This module defines all error codes for the SSO-1 protocol.
//! Each error has a unique code and descriptive message.
//!
//! Error codes are organized by category:
//! - 6000-6009: Authorization errors
//! - 6010-6019: Slot/validity errors
//! - 6020-6029: Market context errors
//! - 6030-6039: Signal assessment errors
//! - 6040-6049: TEE verification errors
//! - 6050-6059: Account state errors
//! - 6060-6069: Protocol configuration errors

use anchor_lang::prelude::*;

/// SSO-1 Protocol Errors
#[error_code]
pub enum SsoError {
    // =========================================================================
    // AUTHORIZATION ERRORS (6000-6009)
    // =========================================================================
    
    /// Provider is not registered in the protocol
    #[msg("Provider is not registered in the protocol")]
    ProviderNotRegistered, // 6000

    /// Signer is not authorized to perform this action
    #[msg("Signer is not authorized to perform this action")]
    Unauthorized, // 6001

    /// Provider account is not active
    #[msg("Provider account is not active")]
    ProviderNotActive, // 6002

    /// Admin authority required for this operation
    #[msg("Admin authority required for this operation")]
    AdminRequired, // 6003

    /// Provider authority mismatch
    #[msg("Provider authority does not match signer")]
    ProviderAuthorityMismatch, // 6004

    // =========================================================================
    // SLOT/VALIDITY ERRORS (6010-6019)
    // =========================================================================

    /// Signal has already expired
    #[msg("Signal has already expired (current_slot > valid_until_slot)")]
    SignalExpired, // 6005

    /// Signal validity period exceeds maximum allowed
    #[msg("Signal validity period exceeds maximum allowed")]
    ValidityPeriodTooLong, // 6006

    /// Signal validity period below minimum required
    #[msg("Signal validity period below minimum required")]
    ValidityPeriodTooShort, // 6007

    /// Invalid slot timestamp (generated_at > valid_until)
    #[msg("Invalid slot timestamp: generated_at_slot exceeds valid_until_slot")]
    InvalidSlotTimestamp, // 6008

    /// Market data is stale (slot drift too high)
    #[msg("Market data is stale: slot drift exceeds maximum allowed")]
    StaleMarketData, // 6009

    // =========================================================================
    // MARKET CONTEXT ERRORS (6020-6029)
    // =========================================================================

    /// Market context validation failed
    #[msg("Market context validation failed")]
    InvalidMarketContext, // 6010

    /// Price cannot be zero
    #[msg("Price cannot be zero")]
    ZeroPrice, // 6011

    /// Source count is below minimum required
    #[msg("Source count is below minimum required")]
    InsufficientSources, // 6012

    /// Source bitmap does not match source count
    #[msg("Source bitmap does not match source count")]
    SourceBitmapMismatch, // 6013

    /// Invalid asset symbol
    #[msg("Invalid asset symbol format")]
    InvalidAssetSymbol, // 6014

    // =========================================================================
    // SIGNAL ASSESSMENT ERRORS (6030-6039)
    // =========================================================================

    /// Signal assessment validation failed
    #[msg("Signal assessment validation failed")]
    InvalidSignalAssessment, // 6015

    /// Confidence score is below minimum threshold
    #[msg("Confidence score is below minimum threshold")]
    InsufficientConfidence, // 6016

    /// Basis point value exceeds maximum (10000)
    #[msg("Basis point value exceeds maximum (10000)")]
    BasisPointOverflow, // 6017

    /// Invalid signal direction value
    #[msg("Invalid signal direction value")]
    InvalidSignalDirection, // 6018

    /// Strength score is invalid
    #[msg("Strength score is invalid (must be 0-10000)")]
    InvalidStrengthScore, // 6019

    // =========================================================================
    // TEE VERIFICATION ERRORS (6040-6049)
    // =========================================================================

    /// TEE attestation verification failed
    #[msg("TEE attestation verification failed")]
    TeeVerificationFailed, // 6020

    /// MR_ENCLAVE not in provider's allowlist
    #[msg("MR_ENCLAVE not in provider's allowlist")]
    EnclaveNotAllowed, // 6021

    /// Invalid TEE signature
    #[msg("Invalid TEE signature")]
    InvalidTeeSignature, // 6022

    /// TEE platform not supported
    #[msg("TEE platform not supported (AMD SEV-SNP required)")]
    UnsupportedTeePlatform, // 6023

    /// TEE receipt timestamp invalid
    #[msg("TEE receipt timestamp is invalid")]
    InvalidTeeTimestamp, // 6024

    /// Report data binding verification failed
    #[msg("Report data binding verification failed")]
    ReportDataMismatch, // 6025

    // =========================================================================
    // ACCOUNT STATE ERRORS (6050-6059)
    // =========================================================================

    /// Signal account is not in correct state
    #[msg("Signal account is not in correct state for this operation")]
    InvalidSignalState, // 6026

    /// Signal account already initialized
    #[msg("Signal account has already been initialized")]
    AlreadyInitialized, // 6027

    /// Signal account not found
    #[msg("Signal account not found")]
    SignalNotFound, // 6028

    /// Signal has already been revoked
    #[msg("Signal has already been revoked")]
    AlreadyRevoked, // 6029

    /// Cannot update expired signal
    #[msg("Cannot update an expired signal")]
    CannotUpdateExpired, // 6030

    /// Account data is corrupted
    #[msg("Account data is corrupted or invalid")]
    CorruptedAccountData, // 6031

    // =========================================================================
    // PROTOCOL CONFIGURATION ERRORS (6060-6069)
    // =========================================================================

    /// Protocol is currently paused
    #[msg("Protocol is currently paused")]
    ProtocolPaused, // 6032

    /// Global config not initialized
    #[msg("Global config has not been initialized")]
    ConfigNotInitialized, // 6033

    /// Invalid configuration parameter
    #[msg("Invalid configuration parameter")]
    InvalidConfigParameter, // 6034

    /// Maximum enclave count exceeded
    #[msg("Maximum enclave count exceeded (max 8)")]
    MaxEnclavesExceeded, // 6035

    /// Enclave already exists in allowlist
    #[msg("Enclave already exists in provider's allowlist")]
    EnclaveAlreadyExists, // 6036

    /// Enclave not found in allowlist
    #[msg("Enclave not found in provider's allowlist")]
    EnclaveNotFound, // 6037

    /// Protocol version mismatch
    #[msg("Protocol version mismatch")]
    VersionMismatch, // 6038

    // =========================================================================
    // GENERAL ERRORS (6070+)
    // =========================================================================

    /// Arithmetic overflow occurred
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow, // 6039

    /// Invalid PDA seeds
    #[msg("Invalid PDA seeds")]
    InvalidPdaSeeds, // 6040

    /// Account already exists
    #[msg("Account already exists")]
    AccountAlreadyExists, // 6041

    /// Operation not allowed
    #[msg("Operation not allowed in current state")]
    OperationNotAllowed, // 6042
}

// =============================================================================
// ERROR RESULT HELPERS
// =============================================================================

/// Helper trait for converting validation results to errors
pub trait IntoSsoError<T> {
    fn require_or(self, error: SsoError) -> Result<T>;
}

impl<T> IntoSsoError<T> for Option<T> {
    fn require_or(self, error: SsoError) -> Result<T> {
        self.ok_or(error.into())
    }
}

/// Helper function to check slot validity
pub fn check_slot_validity(
    current_slot: u64,
    valid_until_slot: u64,
) -> Result<()> {
    require!(
        current_slot <= valid_until_slot,
        SsoError::SignalExpired
    );
    Ok(())
}

/// Helper function to check basis point bounds
pub fn check_bps_bounds(value: u16) -> Result<()> {
    require!(
        value <= 10000,
        SsoError::BasisPointOverflow
    );
    Ok(())
}

/// Helper function to check if protocol is active
pub fn check_protocol_active(is_paused: bool) -> Result<()> {
    require!(!is_paused, SsoError::ProtocolPaused);
    Ok(())
}

// =============================================================================
// UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_slot_validity() {
        // Valid: current <= valid_until
        assert!(check_slot_validity(500, 1000).is_ok());
        assert!(check_slot_validity(1000, 1000).is_ok());
        
        // Invalid: current > valid_until
        assert!(check_slot_validity(1001, 1000).is_err());
    }

    #[test]
    fn test_check_bps_bounds() {
        // Valid
        assert!(check_bps_bounds(0).is_ok());
        assert!(check_bps_bounds(5000).is_ok());
        assert!(check_bps_bounds(10000).is_ok());
        
        // Invalid
        assert!(check_bps_bounds(10001).is_err());
    }

    #[test]
    fn test_check_protocol_active() {
        assert!(check_protocol_active(false).is_ok());
        assert!(check_protocol_active(true).is_err());
    }
}
