# NEX Core Design Principles

1. **Local-First, Cloud-Never Mandatory:** Offline is a first-class state, not an error condition.
2. **Capability-Secured:** No ambient authority. Every action requires a verified capability token.
3. **Transport Independence:** Core logic is ignorant of whether bits travel over fiber, Wi-Fi, or radio.
4. **Deterministic Convergence:** Replicated state converges mathematically without central arbiters.
5. **Zero-Friction Usability:** Cryptography must be invisible, fast, and seamless for non-experts.
6. **Graceful Degradation:** When disconnected from the mesh, local functionality remains 100% operational.
