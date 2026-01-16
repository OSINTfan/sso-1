# Disclaimer

## Not Financial Advice

SSO-1 is infrastructure software. Signals produced by SSO-1 providers are **not financial advice**. Nothing in this protocol, its documentation, or signals derived from it constitutes a recommendation to buy, sell, or hold any asset.

Trading is risky. You can lose money. Make your own decisions.

---

## What SSO-1 Guarantees

The protocol provides cryptographic verification that:

1. **Provenance** - A signal was produced by a specific enclave identified by its `mr_enclave` measurement
2. **Integrity** - The signal was not tampered with between computation and on-chain submission
3. **Freshness** - The signal was submitted within its declared validity window (`current_slot <= valid_until_slot`)

---

## What SSO-1 Does NOT Guarantee

### Signal Accuracy

TEE attestation proves **code ran untampered** - it does not prove the code is **correct, optimal, or profitable**. A buggy model produces attested garbage. A mediocre model produces attested mediocrity.

The protocol verifies *authenticity*, not *quality*.

### Model Correctness

Providers are responsible for their models. The protocol has no mechanism to:
- Validate predictive power
- Penalize bad signals
- Track historical accuracy

### Data Source Accuracy

SSO-1 does not solve the data oracle problem. Market data enters the TEE from external sources. If those sources are wrong, delayed, or manipulated, the signal will reflect that.

Garbage in, garbage out - with attestation.

### TEE Invincibility

AMD SEV-SNP is hardware security, not magic. Known limitations:

- **Side-channel attacks** - Timing, power analysis, and other side-channels may leak information
- **Hardware vulnerabilities** - New exploits are discovered periodically
- **Availability** - The host can refuse to run the enclave (denial of service)
- **Performance constraints** - Memory, CPU, and network limitations apply

The protocol assumes AMD's security model holds. If it doesn't, attestations are meaningless.

### Financial Outcomes

Using SSO-1 signals does not guarantee profits. Signals may be:
- Wrong
- Stale by the time you act
- Correct but unprofitable due to fees, slippage, or timing
- Correct but followed by unexpected market moves

You bear all risk from acting on signals.

---

## Current Implementation Status

This repository contains:

| Component | Status |
|-----------|--------|
| Protocol specification | Complete |
| On-chain program structure | Complete |
| TEE function scaffolding | Complete |
| Scoring logic | **Stub only/in development** (TODOs) |
| Deployed program | **Not deployed/in development/localnet** |
| Audits | **None** |
| Production providers | **None** |

The scoring module (`oracle/offchain/scoring/`) contains placeholder implementations that return default values. Providers must implement their own proprietary logic.

---

## Known Limitations

### Data Oracle Problem

SSO-1 attests that computation occurred correctly. It does not verify that input data (prices, volumes) is accurate. Data source integrity is the provider's responsibility.

### Single TEE Provider

Current implementation uses Switchboard V3 for TEE execution. This is a dependency and potential single point of failure.

### No Reputation System

There is no on-chain mechanism to track provider accuracy or penalize bad signals. Reputation is off-chain and informal.

### Confidence Calibration

Confidence scores (0-100%) are provider-defined. There is no protocol-level calibration or standardization of what confidence levels mean.

---

## Software Provided As-Is

This software is provided "as is", without warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose, and noninfringement.

In no event shall the authors or copyright holders be liable for any claim, damages, or other liability, whether in an action of contract, tort, or otherwise, arising from, out of, or in connection with the software or the use or other dealings in the software.

---

## Your Responsibility

Before using SSO-1 signals:

1. **Verify the provider** - Check their `mr_enclave`, audit status, and reputation
2. **Understand the model** - Know what you're trusting (or that you're trusting blindly)
3. **Check validity** - Always verify `current_slot <= valid_until_slot`
4. **Manage risk** - Never risk more than you can afford to lose
5. **Do your own research** - Signals are inputs, not instructions

---

## Contact

Security vulnerabilities: See [SECURITY.md](SECURITY.md)

General inquiries: [your contact method]


ADDENDUM:

While Claude generated almost the entirety (let's say around ~99%) of this codebase, the architecture and decisions behind it were shaped by two experienced engineers. Nowadays, even the best engineers we know leverage LLMs as a core tool in their workflows. We recognize the code isn't always optimal, which is precisely why we're opening this to the public for real-world feedback and iteration.