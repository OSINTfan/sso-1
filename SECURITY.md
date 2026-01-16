# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.2.x   | :white_check_mark: |
| < 1.2   | :x:                |

## Reporting a Vulnerability

**Do not report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability within SSO-1, please send an email to acephale4w@outlook.com or 4worlds.4w@gmail.com.

Please include:

1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 1 week
- **Resolution Timeline**: Depends on severity

## Severity Levels

| Level | Description | Response |
|-------|-------------|----------|
| Critical | TEE bypass, fund loss | Immediate patch |
| High | Signal manipulation | Priority fix |
| Medium | DOS, information leak | Scheduled fix |
| Low | Minor issues | Normal release |

## Security Considerations

### TEE Attestation

- All signals MUST originate from verified TEE enclaves
- MR_ENCLAVE values MUST be audited before adding to allowlist
- Attestation verification is critical path

### Slot Validity

- Slot-relative validity is a security invariant
- Never bypass `current_slot <= valid_until_slot` check
- Stale signals MUST be rejected

### Key Management

- Never commit keypairs or secrets
- Use environment variables for sensitive data
- Rotate keys regularly in production

## Audit Status

| Component | Auditor | Date | Status |
|-----------|---------|------|--------|
| On-Chain Program | TBD | TBD | Pending |
| Off-Chain Function | TBD | TBD | Pending |

## Bug Bounty

Details coming soon.
