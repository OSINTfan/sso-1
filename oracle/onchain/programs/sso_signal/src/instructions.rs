//! SSO-1 v1.2 Instruction Handlers
//!
//! This module contains all instruction handlers for the SSO-1 protocol.
//! Each instruction is implemented as a function with an associated Context struct.
//!
//! Instructions are organized by category:
//! - Configuration: initialize_config, pause_protocol
//! - Provider Management: register_provider, add_enclave, remove_enclave
//! - Signal Operations: submit_signal, update_signal, revoke_signal

use anchor_lang::prelude::*;

use crate::errors::{check_bps_bounds, check_protocol_active, check_slot_validity, SsoError};
use crate::state::*;

// =============================================================================
// INITIALIZE CONFIG
// =============================================================================

/// Initialize the global protocol configuration
///
/// This instruction creates the GlobalConfig PDA that governs all protocol operations.
/// Can only be called once.
#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Protocol admin (signer + payer)
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Global config PDA
    #[account(
        init,
        payer = admin,
        space = GlobalConfig::LEN,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump
    )]
    pub config: Account<'info, GlobalConfig>,

    /// System program for account creation
    pub system_program: Program<'info, System>,
}

/// Initialize global configuration
pub fn handle_initialize_config(
    ctx: Context<InitializeConfig>,
    min_validity_slots: u64,
    max_validity_slots: u64,
    min_source_count: u8,
    min_confidence_bps: u16,
) -> Result<()> {
    // Validate parameters
    require!(
        min_validity_slots <= max_validity_slots,
        SsoError::InvalidConfigParameter
    );
    check_bps_bounds(min_confidence_bps)?;
    require!(min_source_count >= 1, SsoError::InvalidConfigParameter);

    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.min_validity_slots = min_validity_slots;
    config.max_validity_slots = max_validity_slots;
    config.min_source_count = min_source_count;
    config.min_confidence_bps = min_confidence_bps;
    config.is_paused = false;
    config.protocol_version = SPEC_VERSION as u16;
    config.total_signals = 0;
    config.total_providers = 0;
    config.bump = ctx.bumps.config;
    config._reserved = [0u8; 32];

    msg!("SSO-1 Protocol initialized. Admin: {}", config.admin);
    Ok(())
}

// =============================================================================
// REGISTER PROVIDER
// =============================================================================

/// Register a new signal provider
///
/// Creates a ProviderRegistry PDA that tracks provider authorization and enclave allowlist.
#[derive(Accounts)]
pub struct RegisterProvider<'info> {
    /// Provider authority (signer + payer)
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Provider registry PDA
    #[account(
        init,
        payer = authority,
        space = ProviderRegistry::LEN,
        seeds = [ProviderRegistry::SEED_PREFIX, authority.key().as_ref()],
        bump
    )]
    pub provider: Account<'info, ProviderRegistry>,

    /// Global config (checked for pause state)
    #[account(
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = config.bump,
    )]
    pub config: Account<'info, GlobalConfig>,

    /// System program
    pub system_program: Program<'info, System>,
}

/// Register a new provider
pub fn handle_register_provider(
    ctx: Context<RegisterProvider>,
    name: [u8; 32],
    initial_enclave: Option<[u8; 32]>,
) -> Result<()> {
    check_protocol_active(ctx.accounts.config.is_paused)?;

    let clock = Clock::get()?;
    let provider = &mut ctx.accounts.provider;

    provider.authority = ctx.accounts.authority.key();
    provider.name = name;
    provider.is_active = true;
    provider.signal_count = 0;
    provider.registered_at_slot = clock.slot;
    provider.last_active_slot = clock.slot;
    provider.enclave_count = 0;
    provider.allowed_enclaves = [[0u8; 32]; MAX_ENCLAVES_PER_PROVIDER];
    provider.bump = ctx.bumps.provider;
    provider._reserved = [0u8; 32];

    // Add initial enclave if provided
    if let Some(enclave) = initial_enclave {
        provider.add_enclave(enclave);
    }

    msg!(
        "Provider registered: {}",
        ctx.accounts.authority.key()
    );
    Ok(())
}

// =============================================================================
// SUBMIT SIGNAL
// =============================================================================

/// Submit a new signal
///
/// Creates a SignalAccount PDA containing the verified signal data.
/// Validates TEE attestation and market context freshness.
#[derive(Accounts)]
#[instruction(signal_id: [u8; 32])]
pub struct SubmitSignal<'info> {
    /// Provider authority (signer + payer)
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Provider registry (must be registered and active)
    #[account(
        mut,
        seeds = [ProviderRegistry::SEED_PREFIX, authority.key().as_ref()],
        bump = provider.bump,
        constraint = provider.is_active @ SsoError::ProviderNotActive,
        constraint = provider.authority == authority.key() @ SsoError::ProviderAuthorityMismatch
    )]
    pub provider: Account<'info, ProviderRegistry>,

    /// Signal account PDA
    #[account(
        init,
        payer = authority,
        space = SignalAccount::LEN,
        seeds = [SignalAccount::SEED_PREFIX, authority.key().as_ref(), signal_id.as_ref()],
        bump
    )]
    pub signal: Account<'info, SignalAccount>,

    /// Global config
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = config.bump,
    )]
    pub config: Account<'info, GlobalConfig>,

    /// System program
    pub system_program: Program<'info, System>,
}

/// Submit a new signal
pub fn handle_submit_signal(
    ctx: Context<SubmitSignal>,
    signal_id: [u8; 32],
    market_context: MarketContext,
    signal_assessment: SignalAssessment,
    tee_receipt: TeeReceipt,
) -> Result<()> {
    let config = &ctx.accounts.config;
    let provider = &mut ctx.accounts.provider;
    let clock = Clock::get()?;

    // Check protocol not paused
    check_protocol_active(config.is_paused)?;

    // Validate market context
    require!(
        market_context.validate(),
        SsoError::InvalidMarketContext
    );
    require!(
        market_context.source_count >= config.min_source_count,
        SsoError::InsufficientSources
    );
    require!(
        market_context.is_fresh(clock.slot),
        SsoError::StaleMarketData
    );

    // Validate signal assessment
    require!(
        signal_assessment.validate(clock.slot),
        SsoError::InvalidSignalAssessment
    );
    require!(
        signal_assessment.confidence_bps >= config.min_confidence_bps,
        SsoError::InsufficientConfidence
    );

    // Validate validity period
    let validity_slots = signal_assessment
        .valid_until_slot
        .saturating_sub(clock.slot);
    require!(
        validity_slots >= config.min_validity_slots,
        SsoError::ValidityPeriodTooShort
    );
    require!(
        validity_slots <= config.max_validity_slots,
        SsoError::ValidityPeriodTooLong
    );

    // Verify TEE attestation
    require!(
        tee_receipt.platform == TeePlatform::AmdSevSnp,
        SsoError::UnsupportedTeePlatform
    );
    require!(
        provider.is_enclave_allowed(&tee_receipt.mr_enclave),
        SsoError::EnclaveNotAllowed
    );

    // Initialize signal account
    let signal = &mut ctx.accounts.signal;
    signal.provider = ctx.accounts.authority.key();
    signal.signal_id = signal_id;
    signal.spec_version = SPEC_VERSION;
    signal.status = SignalStatus::Active;
    signal.market_context = market_context;
    signal.signal_assessment = signal_assessment;
    signal.tee_receipt = tee_receipt;
    signal.created_at_slot = clock.slot;
    signal.updated_at_slot = clock.slot;
    signal.update_count = 0;
    signal.bump = ctx.bumps.signal;
    signal._reserved = [0u8; 32];

    // Update provider stats
    provider.signal_count = provider.signal_count.saturating_add(1);
    provider.last_active_slot = clock.slot;

    // Update global stats
    let config = &mut ctx.accounts.config;
    config.total_signals = config.total_signals.saturating_add(1);

    msg!(
        "Signal submitted: {} by provider {}",
        hex::encode(signal_id),
        ctx.accounts.authority.key()
    );

    Ok(())
}

// =============================================================================
// UPDATE SIGNAL
// =============================================================================

/// Update an existing signal
///
/// Updates the market context and signal assessment for an existing signal.
/// Signal must not be expired or revoked.
#[derive(Accounts)]
pub struct UpdateSignal<'info> {
    /// Provider authority (signer)
    pub authority: Signer<'info>,

    /// Provider registry
    #[account(
        mut,
        seeds = [ProviderRegistry::SEED_PREFIX, authority.key().as_ref()],
        bump = provider.bump,
        constraint = provider.is_active @ SsoError::ProviderNotActive,
        constraint = provider.authority == authority.key() @ SsoError::ProviderAuthorityMismatch
    )]
    pub provider: Account<'info, ProviderRegistry>,

    /// Signal account to update
    #[account(
        mut,
        seeds = [SignalAccount::SEED_PREFIX, authority.key().as_ref(), signal.signal_id.as_ref()],
        bump = signal.bump,
        constraint = signal.provider == authority.key() @ SsoError::ProviderAuthorityMismatch,
        constraint = signal.status == SignalStatus::Active @ SsoError::InvalidSignalState
    )]
    pub signal: Account<'info, SignalAccount>,

    /// Global config
    #[account(
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = config.bump,
    )]
    pub config: Account<'info, GlobalConfig>,
}

/// Update an existing signal
pub fn handle_update_signal(
    ctx: Context<UpdateSignal>,
    market_context: MarketContext,
    signal_assessment: SignalAssessment,
    tee_receipt: TeeReceipt,
) -> Result<()> {
    let config = &ctx.accounts.config;
    let provider = &mut ctx.accounts.provider;
    let signal = &mut ctx.accounts.signal;
    let clock = Clock::get()?;

    // Check protocol not paused
    check_protocol_active(config.is_paused)?;

    // Check signal not expired
    check_slot_validity(clock.slot, signal.signal_assessment.valid_until_slot)?;

    // Validate new market context
    require!(
        market_context.validate(),
        SsoError::InvalidMarketContext
    );
    require!(
        market_context.is_fresh(clock.slot),
        SsoError::StaleMarketData
    );

    // Validate new signal assessment
    require!(
        signal_assessment.validate(clock.slot),
        SsoError::InvalidSignalAssessment
    );

    // Verify TEE attestation
    require!(
        tee_receipt.platform == TeePlatform::AmdSevSnp,
        SsoError::UnsupportedTeePlatform
    );
    require!(
        provider.is_enclave_allowed(&tee_receipt.mr_enclave),
        SsoError::EnclaveNotAllowed
    );

    // Update signal
    signal.market_context = market_context;
    signal.signal_assessment = signal_assessment;
    signal.tee_receipt = tee_receipt;
    signal.updated_at_slot = clock.slot;
    signal.update_count = signal.update_count.saturating_add(1);

    // Update provider stats
    provider.last_active_slot = clock.slot;

    msg!(
        "Signal updated: {} (update #{})",
        hex::encode(signal.signal_id),
        signal.update_count
    );

    Ok(())
}

// =============================================================================
// REVOKE SIGNAL
// =============================================================================

/// Revoke an existing signal
///
/// Marks a signal as revoked. Signal can no longer be used.
#[derive(Accounts)]
pub struct RevokeSignal<'info> {
    /// Provider authority (signer)
    pub authority: Signer<'info>,

    /// Signal account to revoke
    #[account(
        mut,
        seeds = [SignalAccount::SEED_PREFIX, authority.key().as_ref(), signal.signal_id.as_ref()],
        bump = signal.bump,
        constraint = signal.provider == authority.key() @ SsoError::ProviderAuthorityMismatch,
        constraint = signal.status == SignalStatus::Active @ SsoError::InvalidSignalState
    )]
    pub signal: Account<'info, SignalAccount>,
}

/// Revoke a signal
pub fn handle_revoke_signal(ctx: Context<RevokeSignal>) -> Result<()> {
    let signal = &mut ctx.accounts.signal;
    let clock = Clock::get()?;

    signal.status = SignalStatus::Revoked;
    signal.updated_at_slot = clock.slot;

    msg!(
        "Signal revoked: {}",
        hex::encode(signal.signal_id)
    );

    Ok(())
}

// =============================================================================
// ADD ENCLAVE
// =============================================================================

/// Add an enclave to provider's allowlist
///
/// Adds a new MR_ENCLAVE value to the provider's allowlist.
/// Maximum 8 enclaves per provider.
#[derive(Accounts)]
pub struct AddEnclave<'info> {
    /// Provider authority (signer)
    pub authority: Signer<'info>,

    /// Provider registry
    #[account(
        mut,
        seeds = [ProviderRegistry::SEED_PREFIX, authority.key().as_ref()],
        bump = provider.bump,
        constraint = provider.authority == authority.key() @ SsoError::ProviderAuthorityMismatch
    )]
    pub provider: Account<'info, ProviderRegistry>,
}

/// Add enclave to allowlist
pub fn handle_add_enclave(
    ctx: Context<AddEnclave>,
    mr_enclave: [u8; 32],
) -> Result<()> {
    let provider = &mut ctx.accounts.provider;

    require!(
        (provider.enclave_count as usize) < MAX_ENCLAVES_PER_PROVIDER,
        SsoError::MaxEnclavesExceeded
    );

    require!(
        !provider.is_enclave_allowed(&mr_enclave),
        SsoError::EnclaveAlreadyExists
    );

    provider.add_enclave(mr_enclave);

    msg!(
        "Enclave added: {} (total: {})",
        hex::encode(mr_enclave),
        provider.enclave_count
    );

    Ok(())
}

// =============================================================================
// REMOVE ENCLAVE
// =============================================================================

/// Remove an enclave from provider's allowlist
#[derive(Accounts)]
pub struct RemoveEnclave<'info> {
    /// Provider authority (signer)
    pub authority: Signer<'info>,

    /// Provider registry
    #[account(
        mut,
        seeds = [ProviderRegistry::SEED_PREFIX, authority.key().as_ref()],
        bump = provider.bump,
        constraint = provider.authority == authority.key() @ SsoError::ProviderAuthorityMismatch
    )]
    pub provider: Account<'info, ProviderRegistry>,
}

/// Remove enclave from allowlist
pub fn handle_remove_enclave(
    ctx: Context<RemoveEnclave>,
    mr_enclave: [u8; 32],
) -> Result<()> {
    let provider = &mut ctx.accounts.provider;

    require!(
        provider.is_enclave_allowed(&mr_enclave),
        SsoError::EnclaveNotFound
    );

    provider.remove_enclave(&mr_enclave);

    msg!(
        "Enclave removed: {} (remaining: {})",
        hex::encode(mr_enclave),
        provider.enclave_count
    );

    Ok(())
}

// =============================================================================
// PAUSE PROTOCOL
// =============================================================================

/// Pause or unpause the protocol (admin only)
#[derive(Accounts)]
pub struct PauseProtocol<'info> {
    /// Admin authority (signer)
    pub admin: Signer<'info>,

    /// Global config
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = config.admin == admin.key() @ SsoError::AdminRequired
    )]
    pub config: Account<'info, GlobalConfig>,
}

/// Pause or unpause protocol
pub fn handle_pause_protocol(
    ctx: Context<PauseProtocol>,
    paused: bool,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.is_paused = paused;

    msg!(
        "Protocol pause state: {}",
        if paused { "PAUSED" } else { "ACTIVE" }
    );

    Ok(())
}

// =============================================================================
// UPDATE CONFIG
// =============================================================================

/// Update protocol configuration (admin only)
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    /// Admin authority (signer)
    pub admin: Signer<'info>,

    /// Global config
    #[account(
        mut,
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = config.admin == admin.key() @ SsoError::AdminRequired
    )]
    pub config: Account<'info, GlobalConfig>,
}

/// Update configuration parameters
pub fn handle_update_config(
    ctx: Context<UpdateConfig>,
    min_validity_slots: Option<u64>,
    max_validity_slots: Option<u64>,
    min_source_count: Option<u8>,
    min_confidence_bps: Option<u16>,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    if let Some(min) = min_validity_slots {
        config.min_validity_slots = min;
    }
    if let Some(max) = max_validity_slots {
        config.max_validity_slots = max;
    }
    if let Some(count) = min_source_count {
        require!(count >= 1, SsoError::InvalidConfigParameter);
        config.min_source_count = count;
    }
    if let Some(conf) = min_confidence_bps {
        check_bps_bounds(conf)?;
        config.min_confidence_bps = conf;
    }

    // Validate consistency
    require!(
        config.min_validity_slots <= config.max_validity_slots,
        SsoError::InvalidConfigParameter
    );

    msg!("Config updated");
    Ok(())
}

// =============================================================================
// DEACTIVATE PROVIDER
// =============================================================================

/// Deactivate a provider (provider or admin)
#[derive(Accounts)]
pub struct DeactivateProvider<'info> {
    /// Authority (provider or admin)
    pub authority: Signer<'info>,

    /// Provider registry to deactivate
    #[account(
        mut,
        seeds = [ProviderRegistry::SEED_PREFIX, provider.authority.as_ref()],
        bump = provider.bump,
    )]
    pub provider: Account<'info, ProviderRegistry>,

    /// Global config (to check admin)
    #[account(
        seeds = [GlobalConfig::SEED_PREFIX],
        bump = config.bump,
    )]
    pub config: Account<'info, GlobalConfig>,
}

/// Deactivate a provider
pub fn handle_deactivate_provider(ctx: Context<DeactivateProvider>) -> Result<()> {
    let authority = ctx.accounts.authority.key();
    let provider = &mut ctx.accounts.provider;
    let config = &ctx.accounts.config;

    // Must be either the provider authority or admin
    require!(
        authority == provider.authority || authority == config.admin,
        SsoError::Unauthorized
    );

    provider.is_active = false;

    msg!("Provider deactivated: {}", provider.authority);
    Ok(())
}
