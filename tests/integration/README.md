# SSO-1 Integration Tests

These tests verify the complete SSO-1 protocol flow on-chain.

## Prerequisites

1. **Anchor CLI** installed (`anchor --version`)
2. **Solana CLI** installed (`solana --version`)
3. **Python 3.10+** with pip

## Setup

```bash
# Install Python dependencies
cd tests/integration
pip install -r requirements.txt

# Build the program and generate IDL
cd ../../oracle/onchain
anchor build
```

## Running Tests

### Option 1: Local Validator (Recommended for Development)

```bash
# Terminal 1: Start local validator
solana-test-validator

# Terminal 2: Deploy and test
cd oracle/onchain
anchor deploy --provider.cluster localnet

cd ../..
pytest tests/integration/ -v
```

### Option 2: Anchor Test (All-in-One)

```bash
cd oracle/onchain
anchor test
```

This will:
1. Start a local validator
2. Deploy the program
3. Run tests
4. Shut down the validator

### Option 3: Devnet

```bash
# Ensure you have devnet SOL
solana airdrop 2 --url devnet

# Deploy to devnet
cd oracle/onchain
anchor deploy --provider.cluster devnet

# Run tests against devnet
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com pytest tests/integration/ -v
```

## Test Coverage

| Test Class | What It Tests |
|------------|---------------|
| `TestProtocolInitialization` | Config account creation |
| `TestProviderRegistration` | Provider PDA creation, enclave allowlist |
| `TestSignalSubmission` | Signal creation, validation, TEE verification |
| `TestSignalRevocation` | Signal revocation flow |
| `TestSlotValidity` | Slot-relative validity enforcement |
| `TestEndToEnd` | Complete lifecycle |

## Expected Results

All tests should pass:

```
tests/integration/test_signal_oracle.py::TestProtocolInitialization::test_initialize_config PASSED
tests/integration/test_signal_oracle.py::TestProviderRegistration::test_register_provider PASSED
tests/integration/test_signal_oracle.py::TestSignalSubmission::test_submit_signal_success PASSED
tests/integration/test_signal_oracle.py::TestSignalSubmission::test_submit_expired_signal_fails PASSED
tests/integration/test_signal_oracle.py::TestSignalSubmission::test_submit_low_confidence_fails PASSED
tests/integration/test_signal_oracle.py::TestSignalSubmission::test_submit_unknown_enclave_fails PASSED
tests/integration/test_signal_oracle.py::TestSignalRevocation::test_revoke_signal PASSED
tests/integration/test_signal_oracle.py::TestSlotValidity::test_validity_period_too_short PASSED
tests/integration/test_signal_oracle.py::TestSlotValidity::test_validity_period_too_long PASSED
tests/integration/test_signal_oracle.py::TestEndToEnd::test_full_signal_lifecycle PASSED
```

## Troubleshooting

### "IDL not found"

Run `anchor build` first to generate the IDL.

### "Insufficient funds"

Request an airdrop: `solana airdrop 2`

### "Connection refused"

Ensure `solana-test-validator` is running on port 8899.

### "Account already in use"

Tests are idempotent. Use `solana-test-validator --reset` to clear state.
