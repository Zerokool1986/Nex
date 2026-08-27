# Security Policy

## Reporting a Vulnerability

The NEX team takes security seriously. If you discover a security vulnerability, we appreciate your help in disclosing it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of these methods:

1. **GitHub Private Vulnerability Reporting** — Use GitHub's [private vulnerability reporting](https://github.com/Zerokool1986/Nex/security/advisories/new) feature
2. **Direct contact** — Reach out to the maintainers privately through the repository

### What to Include

- Description of the vulnerability
- Steps to reproduce (proof of concept if possible)
- Potential impact assessment
- Any suggested fixes

### Response Timeline

- **Acknowledgment**: Within 48 hours of report
- **Assessment**: Within 7 days
- **Fix timeline**: Depends on severity, typically within 30 days for critical issues

## Scope

The following areas are in scope for security reports:

| Component | Description |
|---|---|
| **Identity & Cryptography** | Ed25519 key handling, capability token verification, Shamir recovery |
| **State Integrity** | DAG causal ordering, SMT commitments, checkpoint atomicity |
| **Wire Protocol** | `NEX/WIRE/v1` frame parsing, header validation, buffer overflows |
| **Storage** | WAL journal integrity, CAS chunk verification, state.db corruption |
| **Synchronization** | Anti-entropy protocol vulnerabilities, state injection attacks |
| **Transport** | TCP/Reticulum/QUIC adapter vulnerabilities, MitM vectors |
| **FFI Boundary** | C ABI / JNI bridge safety, memory handling, use-after-free |

## Threat Model

NEX's formal threat model is documented in [`NEX/00_CONSTITUTION/NEX-05_SECURITY_THREAT_MODEL.md`](NEX/00_CONSTITUTION/NEX-05_SECURITY_THREAT_MODEL.md).

Key security properties enforced by the platform:

- **Zero ambient authority** — Every action requires a cryptographically signed capability token
- **Local-first encryption** — Data is encrypted with keys only the owner controls
- **Sovereign identity** — Ed25519 key pairs with no external identity provider dependency
- **Transport independence** — Security properties hold regardless of transport conduit
- **Crash consistency** — Two-phase checkpointing with WAL guarantees no silent data corruption

## Supported Versions

| Version | Supported |
|---|---|
| `0.1.x` (current development) | ✅ Active |

## Acknowledgments

We appreciate the security research community's efforts in helping keep NEX and its users safe. Responsible reporters will be credited in the changelog (unless they prefer anonymity).
