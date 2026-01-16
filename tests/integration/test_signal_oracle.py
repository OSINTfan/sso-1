"""
SSO-1 Integration Tests

Verifies the complete flow from signal generation to on-chain verification.

Prerequisites:
- Anchor CLI installed
- Solana CLI installed  
- Local validator OR devnet access

Run with:
    anchor test                           # Uses local validator
    anchor test --provider.cluster devnet # Uses devnet
"""

from __future__ import annotations

import asyncio
import json
import os
import struct
import hashlib
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, Tuple
from base64 import b64decode

import pytest
from solders.keypair import Keypair
from solders.pubkey import Pubkey
from solders.system_program import ID as SYSTEM_PROGRAM_ID
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from anchorpy import Provider, Wallet, Program, Idl, Context



# =============================================================================
# Configuration
# =============================================================================

# Program ID - update after deployment
PROGRAM_ID = Pubkey.from_string("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS")

# Test constants
MIN_VALIDITY_SLOTS = 10
MAX_VALIDITY_SLOTS = 1000
MIN_SOURCE_COUNT = 1
MIN_CONFIDENCE = 5000  # 50%


@dataclass
class TestConfig:
    """Test configuration."""
    
    rpc_url: str = "http://localhost:8899"
    program_id: Pubkey = PROGRAM_ID
    
    @classmethod
    def from_env(cls) -> "TestConfig":
        return cls(
            rpc_url=os.getenv("ANCHOR_PROVIDER_URL", "http://localhost:8899"),
            program_id=Pubkey.from_string(
                os.getenv("SSO_PROGRAM_ID", str(PROGRAM_ID))
            ),
        )


# =============================================================================
# Test Data Builders
# =============================================================================

@dataclass
class MarketContextBuilder:
    """Builder for MarketContext test data."""
    
    timestamp: int = 0
    captured_at_slot: int = 0
    price_usd: int = 50_000_00000000  # $50,000.00
    volume_24h_usd: int = 1_000_000_000000  # $1B
    market_cap_usd: int = 1_000_000_000_000000  # $1T
    price_change_24h_bps: int = 250  # +2.5%
    spread_bps: int = 10  # 0.1%
    depth_2pct_usd: int = 10_000_000_000000  # $10M
    source_count: int = 3
    source_bitmap: int = 0b111
    asset_symbol: bytes = b"BTC\x00\x00\x00\x00\x00"
    
    def with_slot(self, slot: int) -> "MarketContextBuilder":
        self.captured_at_slot = slot
        self.timestamp = int(time.time())
        return self
    
    def with_sources(self, count: int) -> "MarketContextBuilder":
        self.source_count = count
        self.source_bitmap = (1 << count) - 1
        return self
    
    def build(self) -> dict:
        return {
            "timestamp": self.timestamp,
            "captured_at_slot": self.captured_at_slot,
            "price_usd": self.price_usd,
            "volume_24h_usd": self.volume_24h_usd,
            "market_cap_usd": self.market_cap_usd,
            "price_change_24h_bps": self.price_change_24h_bps,
            "spread_bps": self.spread_bps,
            "depth_2pct_usd": self.depth_2pct_usd,
            "source_count": self.source_count,
            "source_bitmap": self.source_bitmap,
            "asset_symbol": list(self.asset_symbol),
            "reserved": [0] * 32,
        }


@dataclass 
class SignalAssessmentBuilder:
    """Builder for SignalAssessment test data."""
    
    direction: int = 1  # Long
    strength: int = 7500  # 75%
    confidence: int = 8000  # 80%
    time_horizon_slots: int = 100
    valid_until_slot: int = 0
    generated_at_slot: int = 0
    risk_score: int = 3000  # 30%
    suggested_size_bps: int = 500  # 5%
    model_version: int = 1
    model_params_hash: bytes = b"\x00" * 32
    
    def with_validity(self, current_slot: int, duration: int = 100) -> "SignalAssessmentBuilder":
        self.generated_at_slot = current_slot
        self.valid_until_slot = current_slot + duration
        return self
    
    def with_expired(self, current_slot: int) -> "SignalAssessmentBuilder":
        """Create an already-expired signal."""
        self.generated_at_slot = current_slot - 200
        self.valid_until_slot = current_slot - 100
        return self
    
    def with_confidence(self, confidence: int) -> "SignalAssessmentBuilder":
        self.confidence = confidence
        return self
    
    def build(self) -> dict:
        return {
            "direction": {"long": {}},  # Anchor enum format
            "strength": self.strength,
            "confidence": self.confidence,
            "time_horizon_slots": self.time_horizon_slots,
            "valid_until_slot": self.valid_until_slot,
            "generated_at_slot": self.generated_at_slot,
            "risk_score": self.risk_score,
            "suggested_size_bps": self.suggested_size_bps,
            "model_version": self.model_version,
            "model_params_hash": list(self.model_params_hash),
            "reserved": [0] * 16,
        }


@dataclass
class TeeReceiptBuilder:
    """Builder for TeeReceipt test data."""
    
    mr_enclave: bytes = b"\x00" * 32
    mr_signer: bytes = b"\x00" * 32
    enclave_keypair: Optional[Keypair] = None
    attestation_timestamp: int = 0
    platform: int = 1  # AmdSevSnp
    svn: int = 1
    
    def __post_init__(self):
        if self.enclave_keypair is None:
            self.enclave_keypair = Keypair()
    
    def with_enclave(self, mr_enclave: bytes) -> "TeeReceiptBuilder":
        self.mr_enclave = mr_enclave
        return self
    
    def sign(self, signal_id: bytes, market_context: dict, assessment: dict) -> "TeeReceiptBuilder":
        """Sign the signal data with enclave keypair."""
        self.attestation_timestamp = int(time.time())
        return self
    
    def build(self, signal_id: bytes, provider: Pubkey) -> dict:
        """Build TEE receipt with signature."""
        # Compute message hash
        message = self._compute_message(signal_id, provider)
        
        # Sign with enclave keypair
        signature = self.enclave_keypair.sign_message(message)
        
        # Compute report data
        report_data = self._compute_report_data(signal_id, provider)
        
        return {
            "mr_enclave": list(self.mr_enclave),
            "mr_signer": list(self.mr_signer),
            "enclave_signature": list(bytes(signature)),
            "enclave_pubkey": list(bytes(self.enclave_keypair.pubkey())),
            "report_data": list(report_data),
            "attestation_timestamp": self.attestation_timestamp or int(time.time()),
            "platform": {"amd_sev_snp": {}},
            "svn": self.svn,
            "reserved": [0] * 13,
        }
    
    def _compute_message(self, signal_id: bytes, provider: Pubkey) -> bytes:
        """Compute the message that needs to be signed."""
        # This must match the on-chain construct_signing_message function
        return hashlib.sha256(signal_id + bytes(provider)).digest()
    
    def _compute_report_data(self, signal_id: bytes, provider: Pubkey) -> bytes:
        """Compute expected report data."""
        h = hashlib.sha256(signal_id + bytes(provider)).digest()
        return h + (b"\x00" * 32)  # 64 bytes total


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture(scope="session")
def event_loop():
    """Create event loop for async tests."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@pytest.fixture(scope="session")
def config() -> TestConfig:
    """Load test configuration."""
    return TestConfig.from_env()


@pytest.fixture(scope="session")
async def provider(config: TestConfig) -> Provider:
    """Create Anchor provider."""
    client = AsyncClient(config.rpc_url, commitment=Confirmed)
    
    # Load or generate wallet
    wallet_path = Path.home() / ".config" / "solana" / "id.json"
    if wallet_path.exists():
        with open(wallet_path) as f:
            keypair_bytes = bytes(json.load(f))
            wallet = Wallet(Keypair.from_bytes(keypair_bytes))
    else:
        wallet = Wallet.local()
    
    return Provider(client, wallet)


@pytest.fixture(scope="session")
async def program(provider: Provider, config: TestConfig) -> Program:
    """Load SSO-1 program."""
    # Load IDL from target directory
    idl_path = Path(__file__).parent.parent.parent / "oracle" / "onchain" / "target" / "idl" / "sso_signal.json"
    
    if not idl_path.exists():
        pytest.skip(f"IDL not found at {idl_path}. Run 'anchor build' first.")
    
    with open(idl_path) as f:
        idl = Idl.from_json(f.read())
    
    return Program(idl, config.program_id, provider)


@pytest.fixture
async def current_slot(provider: Provider) -> int:
    """Get current slot from cluster."""
    resp = await provider.connection.get_slot()
    return resp.value


@pytest.fixture
def test_signal_id() -> bytes:
    """Generate unique signal ID for test."""
    return hashlib.sha256(f"test-{time.time_ns()}".encode()).digest()


@pytest.fixture
def test_mr_enclave() -> bytes:
    """Generate test MR_ENCLAVE value."""
    return hashlib.sha256(b"test-enclave-v1").digest()


# =============================================================================
# PDA Helpers
# =============================================================================

def derive_config_pda(program_id: Pubkey) -> Tuple[Pubkey, int]:
    """Derive config PDA."""
    return Pubkey.find_program_address([b"config"], program_id)


def derive_provider_pda(program_id: Pubkey, authority: Pubkey) -> Tuple[Pubkey, int]:
    """Derive provider registry PDA."""
    return Pubkey.find_program_address(
        [b"provider", bytes(authority)],
        program_id
    )


def derive_signal_pda(program_id: Pubkey, provider: Pubkey, signal_id: bytes) -> Tuple[Pubkey, int]:
    """Derive signal account PDA."""
    return Pubkey.find_program_address(
        [b"signal", bytes(provider), signal_id],
        program_id
    )


# =============================================================================
# Test: Protocol Initialization
# =============================================================================

class TestProtocolInitialization:
    """Tests for protocol setup."""
    
    @pytest.mark.asyncio
    async def test_initialize_config(self, program: Program, provider: Provider):
        """Test that config can be initialized."""
        config_pda, _ = derive_config_pda(program.program_id)
        
        # Check if already initialized
        config_account = await program.account["GlobalConfig"].fetch_nullable(config_pda)
        
        if config_account is not None:
            pytest.skip("Config already initialized")
        
        # Initialize config
        tx = await program.rpc["initialize_config"](
            MIN_VALIDITY_SLOTS,
            MAX_VALIDITY_SLOTS,
            MIN_SOURCE_COUNT,
            MIN_CONFIDENCE,
            ctx=Context(
                accounts={
                    "config": config_pda,
                    "admin": provider.wallet.public_key,
                    "system_program": SYSTEM_PROGRAM_ID,
                },
                signers=[],
            ),
        )
        
        # Verify
        config_account = await program.account["GlobalConfig"].fetch(config_pda)
        assert config_account.admin == provider.wallet.public_key
        assert config_account.min_validity_slots == MIN_VALIDITY_SLOTS
        assert config_account.max_validity_slots == MAX_VALIDITY_SLOTS
        assert config_account.is_paused == False
    
    @pytest.mark.asyncio
    async def test_initialize_config_twice_fails(self, program: Program, provider: Provider):
        """Test that double initialization fails."""
        config_pda, _ = derive_config_pda(program.program_id)
        
        # Ensure initialized first
        config_account = await program.account["GlobalConfig"].fetch_nullable(config_pda)
        if config_account is None:
            await program.rpc["initialize_config"](
                MIN_VALIDITY_SLOTS,
                MAX_VALIDITY_SLOTS,
                MIN_SOURCE_COUNT,
                MIN_CONFIDENCE,
                ctx=Context(
                    accounts={
                        "config": config_pda,
                        "admin": provider.wallet.public_key,
                        "system_program": SYSTEM_PROGRAM_ID,
                    },
                ),
            )
        
        # Try to initialize again - should fail
        with pytest.raises(Exception) as exc_info:
            await program.rpc["initialize_config"](
                MIN_VALIDITY_SLOTS,
                MAX_VALIDITY_SLOTS,
                MIN_SOURCE_COUNT,
                MIN_CONFIDENCE,
                ctx=Context(
                    accounts={
                        "config": config_pda,
                        "admin": provider.wallet.public_key,
                        "system_program": SYSTEM_PROGRAM_ID,
                    },
                ),
            )
        
        # Anchor returns account already initialized error
        assert "already in use" in str(exc_info.value).lower() or "0x0" in str(exc_info.value)


# =============================================================================
# Test: Provider Registration
# =============================================================================

class TestProviderRegistration:
    """Tests for provider registration."""
    
    @pytest.mark.asyncio
    async def test_register_provider(
        self, 
        program: Program, 
        provider: Provider,
        test_mr_enclave: bytes,
    ):
        """Test that a provider can register."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        # Check if already registered
        provider_account = await program.account["ProviderRegistry"].fetch_nullable(provider_pda)
        if provider_account is not None:
            pytest.skip("Provider already registered")
        
        # Register
        name = b"TestProvider\x00" + b"\x00" * 19  # 32 bytes, null-padded
        
        tx = await program.rpc["register_provider"](
            list(name),
            list(test_mr_enclave),
            ctx=Context(
                accounts={
                    "config": config_pda,
                    "provider_registry": provider_pda,
                    "authority": provider.wallet.public_key,
                    "system_program": SYSTEM_PROGRAM_ID,
                },
            ),
        )
        
        # Verify
        provider_account = await program.account["ProviderRegistry"].fetch(provider_pda)
        assert provider_account.authority == provider.wallet.public_key
        assert provider_account.is_active == True
        assert provider_account.signal_count == 0
        assert bytes(provider_account.allowed_enclaves[0]) == test_mr_enclave
    
    @pytest.mark.asyncio
    async def test_add_enclave(
        self,
        program: Program,
        provider: Provider,
        test_mr_enclave: bytes,
    ):
        """Test adding an enclave to allowlist."""
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        # Ensure registered
        provider_account = await program.account["ProviderRegistry"].fetch_nullable(provider_pda)
        if provider_account is None:
            pytest.skip("Provider not registered")
        
        # Add new enclave
        new_enclave = hashlib.sha256(b"test-enclave-v2").digest()
        
        initial_count = len(provider_account.allowed_enclaves)
        
        tx = await program.rpc["add_enclave"](
            list(new_enclave),
            ctx=Context(
                accounts={
                    "provider_registry": provider_pda,
                    "authority": provider.wallet.public_key,
                },
            ),
        )
        
        # Verify
        provider_account = await program.account["ProviderRegistry"].fetch(provider_pda)
        assert len(provider_account.allowed_enclaves) == initial_count + 1


# =============================================================================
# Test: Signal Submission
# =============================================================================

class TestSignalSubmission:
    """Tests for signal submission."""
    
    @pytest.mark.asyncio
    async def test_submit_signal_success(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
        test_signal_id: bytes,
        test_mr_enclave: bytes,
    ):
        """Test successful signal submission."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, test_signal_id)
        
        # Ensure provider registered
        provider_account = await program.account["ProviderRegistry"].fetch_nullable(provider_pda)
        if provider_account is None:
            pytest.skip("Provider not registered - run test_register_provider first")
        
        # Build test data
        market_context = MarketContextBuilder().with_slot(current_slot).with_sources(3).build()
        assessment = SignalAssessmentBuilder().with_validity(current_slot, 100).build()
        
        # Build TEE receipt with proper signature
        tee_builder = TeeReceiptBuilder().with_enclave(test_mr_enclave)
        tee_receipt = tee_builder.build(test_signal_id, provider.wallet.public_key)
        
        # Submit signal
        tx = await program.rpc["submit_signal"](
            list(test_signal_id),
            market_context,
            assessment,
            tee_receipt,
            ctx=Context(
                accounts={
                    "config": config_pda,
                    "provider_registry": provider_pda,
                    "signal_account": signal_pda,
                    "provider": provider.wallet.public_key,
                    "system_program": SYSTEM_PROGRAM_ID,
                },
            ),
        )
        
        # Verify signal stored
        signal_account = await program.account["SignalAccount"].fetch(signal_pda)
        assert signal_account.provider == provider.wallet.public_key
        assert bytes(signal_account.signal_id) == test_signal_id
        assert signal_account.status.active is not None  # Anchor enum
        assert signal_account.signal_assessment.valid_until_slot == current_slot + 100
    
    @pytest.mark.asyncio
    async def test_submit_expired_signal_fails(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
        test_mr_enclave: bytes,
    ):
        """Test that expired signals are rejected."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        # Unique signal ID
        signal_id = hashlib.sha256(f"expired-{time.time_ns()}".encode()).digest()
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, signal_id)
        
        # Build expired signal
        market_context = MarketContextBuilder().with_slot(current_slot).build()
        assessment = SignalAssessmentBuilder().with_expired(current_slot).build()
        
        tee_builder = TeeReceiptBuilder().with_enclave(test_mr_enclave)
        tee_receipt = tee_builder.build(signal_id, provider.wallet.public_key)
        
        # Should fail with SignalExpired
        with pytest.raises(Exception) as exc_info:
            await program.rpc["submit_signal"](
                list(signal_id),
                market_context,
                assessment,
                tee_receipt,
                ctx=Context(
                    accounts={
                        "config": config_pda,
                        "provider_registry": provider_pda,
                        "signal_account": signal_pda,
                        "provider": provider.wallet.public_key,
                        "system_program": SYSTEM_PROGRAM_ID,
                    },
                ),
            )
        
        assert "SignalExpired" in str(exc_info.value) or "6005" in str(exc_info.value)
    
    @pytest.mark.asyncio
    async def test_submit_low_confidence_fails(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
        test_mr_enclave: bytes,
    ):
        """Test that low confidence signals are rejected."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        signal_id = hashlib.sha256(f"lowconf-{time.time_ns()}".encode()).digest()
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, signal_id)
        
        # Build signal with low confidence
        market_context = MarketContextBuilder().with_slot(current_slot).build()
        assessment = SignalAssessmentBuilder().with_validity(current_slot, 100).with_confidence(1000).build()  # 10% < 50% minimum
        
        tee_builder = TeeReceiptBuilder().with_enclave(test_mr_enclave)
        tee_receipt = tee_builder.build(signal_id, provider.wallet.public_key)
        
        with pytest.raises(Exception) as exc_info:
            await program.rpc["submit_signal"](
                list(signal_id),
                market_context,
                assessment,
                tee_receipt,
                ctx=Context(
                    accounts={
                        "config": config_pda,
                        "provider_registry": provider_pda,
                        "signal_account": signal_pda,
                        "provider": provider.wallet.public_key,
                        "system_program": SYSTEM_PROGRAM_ID,
                    },
                ),
            )
        
        assert "ConfidenceBelowMinimum" in str(exc_info.value) or "6019" in str(exc_info.value)
    
    @pytest.mark.asyncio
    async def test_submit_unknown_enclave_fails(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
    ):
        """Test that unknown MR_ENCLAVE is rejected."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        signal_id = hashlib.sha256(f"badenclave-{time.time_ns()}".encode()).digest()
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, signal_id)
        
        # Use unknown enclave
        unknown_enclave = hashlib.sha256(b"unknown-malicious-enclave").digest()
        
        market_context = MarketContextBuilder().with_slot(current_slot).build()
        assessment = SignalAssessmentBuilder().with_validity(current_slot, 100).build()
        
        tee_builder = TeeReceiptBuilder().with_enclave(unknown_enclave)
        tee_receipt = tee_builder.build(signal_id, provider.wallet.public_key)
        
        with pytest.raises(Exception) as exc_info:
            await program.rpc["submit_signal"](
                list(signal_id),
                market_context,
                assessment,
                tee_receipt,
                ctx=Context(
                    accounts={
                        "config": config_pda,
                        "provider_registry": provider_pda,
                        "signal_account": signal_pda,
                        "provider": provider.wallet.public_key,
                        "system_program": SYSTEM_PROGRAM_ID,
                    },
                ),
            )
        
        assert "EnclaveNotAllowed" in str(exc_info.value) or "6021" in str(exc_info.value)


# =============================================================================
# Test: Signal Revocation
# =============================================================================

class TestSignalRevocation:
    """Tests for signal revocation."""
    
    @pytest.mark.asyncio
    async def test_revoke_signal(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
        test_mr_enclave: bytes,
    ):
        """Test that provider can revoke their signal."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        # Create signal to revoke
        signal_id = hashlib.sha256(f"revoke-{time.time_ns()}".encode()).digest()
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, signal_id)
        
        # Submit signal first
        market_context = MarketContextBuilder().with_slot(current_slot).build()
        assessment = SignalAssessmentBuilder().with_validity(current_slot, 100).build()
        tee_builder = TeeReceiptBuilder().with_enclave(test_mr_enclave)
        tee_receipt = tee_builder.build(signal_id, provider.wallet.public_key)
        
        await program.rpc["submit_signal"](
            list(signal_id),
            market_context,
            assessment,
            tee_receipt,
            ctx=Context(
                accounts={
                    "config": config_pda,
                    "provider_registry": provider_pda,
                    "signal_account": signal_pda,
                    "provider": provider.wallet.public_key,
                    "system_program": SYSTEM_PROGRAM_ID,
                },
            ),
        )
        
        # Revoke
        await program.rpc["revoke_signal"](
            list(signal_id),
            ctx=Context(
                accounts={
                    "signal_account": signal_pda,
                    "provider": provider.wallet.public_key,
                },
            ),
        )
        
        # Verify revoked
        signal_account = await program.account["SignalAccount"].fetch(signal_pda)
        assert signal_account.status.revoked is not None


# =============================================================================
# Test: Slot Validity Invariant
# =============================================================================

class TestSlotValidity:
    """Tests for slot-relative validity enforcement."""
    
    @pytest.mark.asyncio
    async def test_validity_period_too_short(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
        test_mr_enclave: bytes,
    ):
        """Test that too-short validity period is rejected."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        signal_id = hashlib.sha256(f"short-{time.time_ns()}".encode()).digest()
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, signal_id)
        
        # Validity of only 5 slots (min is 10)
        market_context = MarketContextBuilder().with_slot(current_slot).build()
        assessment = SignalAssessmentBuilder().with_validity(current_slot, 5).build()
        
        tee_builder = TeeReceiptBuilder().with_enclave(test_mr_enclave)
        tee_receipt = tee_builder.build(signal_id, provider.wallet.public_key)
        
        with pytest.raises(Exception) as exc_info:
            await program.rpc["submit_signal"](
                list(signal_id),
                market_context,
                assessment,
                tee_receipt,
                ctx=Context(
                    accounts={
                        "config": config_pda,
                        "provider_registry": provider_pda,
                        "signal_account": signal_pda,
                        "provider": provider.wallet.public_key,
                        "system_program": SYSTEM_PROGRAM_ID,
                    },
                ),
            )
        
        assert "ValidityPeriodTooShort" in str(exc_info.value) or "6006" in str(exc_info.value)
    
    @pytest.mark.asyncio
    async def test_validity_period_too_long(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
        test_mr_enclave: bytes,
    ):
        """Test that too-long validity period is rejected."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        signal_id = hashlib.sha256(f"long-{time.time_ns()}".encode()).digest()
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, signal_id)
        
        # Validity of 2000 slots (max is 1000)
        market_context = MarketContextBuilder().with_slot(current_slot).build()
        assessment = SignalAssessmentBuilder().with_validity(current_slot, 2000).build()
        
        tee_builder = TeeReceiptBuilder().with_enclave(test_mr_enclave)
        tee_receipt = tee_builder.build(signal_id, provider.wallet.public_key)
        
        with pytest.raises(Exception) as exc_info:
            await program.rpc["submit_signal"](
                list(signal_id),
                market_context,
                assessment,
                tee_receipt,
                ctx=Context(
                    accounts={
                        "config": config_pda,
                        "provider_registry": provider_pda,
                        "signal_account": signal_pda,
                        "provider": provider.wallet.public_key,
                        "system_program": SYSTEM_PROGRAM_ID,
                    },
                ),
            )
        
        assert "ValidityPeriodTooLong" in str(exc_info.value) or "6007" in str(exc_info.value)


# =============================================================================
# Test: End-to-End Flow
# =============================================================================

class TestEndToEnd:
    """Complete integration tests."""
    
    @pytest.mark.asyncio
    async def test_full_signal_lifecycle(
        self,
        program: Program,
        provider: Provider,
        current_slot: int,
        test_mr_enclave: bytes,
    ):
        """Test complete lifecycle: register → submit → read → update → revoke."""
        config_pda, _ = derive_config_pda(program.program_id)
        provider_pda, _ = derive_provider_pda(program.program_id, provider.wallet.public_key)
        
        signal_id = hashlib.sha256(f"lifecycle-{time.time_ns()}".encode()).digest()
        signal_pda, _ = derive_signal_pda(program.program_id, provider.wallet.public_key, signal_id)
        
        # 1. Ensure config exists
        config_account = await program.account["GlobalConfig"].fetch_nullable(config_pda)
        assert config_account is not None, "Config not initialized"
        
        # 2. Ensure provider registered  
        provider_account = await program.account["ProviderRegistry"].fetch_nullable(provider_pda)
        assert provider_account is not None, "Provider not registered"
        
        # 3. Submit signal
        market_context = MarketContextBuilder().with_slot(current_slot).build()
        assessment = SignalAssessmentBuilder().with_validity(current_slot, 100).build()
        tee_builder = TeeReceiptBuilder().with_enclave(test_mr_enclave)
        tee_receipt = tee_builder.build(signal_id, provider.wallet.public_key)
        
        await program.rpc["submit_signal"](
            list(signal_id),
            market_context,
            assessment,
            tee_receipt,
            ctx=Context(
                accounts={
                    "config": config_pda,
                    "provider_registry": provider_pda,
                    "signal_account": signal_pda,
                    "provider": provider.wallet.public_key,
                    "system_program": SYSTEM_PROGRAM_ID,
                },
            ),
        )
        
        # 4. Read and verify signal
        signal_account = await program.account["SignalAccount"].fetch(signal_pda)
        assert signal_account.status.active is not None
        assert signal_account.signal_assessment.confidence == 8000
        
        # 5. Update signal
        new_slot = current_slot + 10
        new_assessment = SignalAssessmentBuilder().with_validity(new_slot, 150).with_confidence(9000).build()
        new_market = MarketContextBuilder().with_slot(new_slot).build()
        new_tee = TeeReceiptBuilder().with_enclave(test_mr_enclave).build(signal_id, provider.wallet.public_key)
        
        await program.rpc["update_signal"](
            list(signal_id),
            new_market,
            new_assessment,
            new_tee,
            ctx=Context(
                accounts={
                    "config": config_pda,
                    "provider_registry": provider_pda,
                    "signal_account": signal_pda,
                    "provider": provider.wallet.public_key,
                },
            ),
        )
        
        # Verify update
        signal_account = await program.account["SignalAccount"].fetch(signal_pda)
        assert signal_account.signal_assessment.confidence == 9000
        
        # 6. Revoke signal
        await program.rpc["revoke_signal"](
            list(signal_id),
            ctx=Context(
                accounts={
                    "signal_account": signal_pda,
                    "provider": provider.wallet.public_key,
                },
            ),
        )
        
        # Verify revoked
        signal_account = await program.account["SignalAccount"].fetch(signal_pda)
        assert signal_account.status.revoked is not None


# =============================================================================
# Run Configuration
# =============================================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
