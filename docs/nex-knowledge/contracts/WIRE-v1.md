# NEX/WIRE/v1: Binary Wire Protocol Specification

**Authority:** NEX Frozen Protocol Contract (Level 2)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-WIRE-v1.md`  
**Status:** STRICTLY FROZEN & IMMUTABLE  

---

## 1. Fixed Header Layout (48 Bytes Exact)

`[DIRECT SOURCE FACT]`
```text
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Magic: "NEXW" (0x4E455857)    | Protocol Version: 0x0001      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Message Type (16 bits)        | Flags / Reserved (16 bits)    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Payload Length (32 bits, max 2MB = 0x00200000)                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Session Nonce / Ephemeral Counter (64 bits)                   |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Sender Actor ID Digest (128 bits)                             |
|                                                               |
|                                                               |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Payload Checksum: SHA256(Payload)[0..16] (128 bits)           |
|                                                               |
|                                                               |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Encrypted Payload (Variable: Length bytes)                    |
+---------------------------------------------------------------+
```

---

## 2. Field Specifications

`[DIRECT SOURCE FACT]`
- **Magic (32 bits / 4 Bytes):** Fixed ASCII sequence `NEXW` (`0x4E455857`). Packets without this prefix must be rejected immediately at ingress without parsing.
- **Protocol Version (16 bits / 2 Bytes):** Fixed `0x0001`.
- **Message Type (16 bits / 2 Bytes):** Identifies the inner protocol subsystem.
- **Flags / Reserved (16 bits / 2 Bytes):** Framing flags and alignment padding.
- **Payload Length (32 bits / 4 Bytes):** Length of payload in bytes. Hard upper limit: 2,097,152 bytes (2 MB).
- **Session Nonce (64 bits / 8 Bytes):** Ephemeral counter preventing replay attacks.
- **Sender Actor ID Digest (128 bits / 16 Bytes):** First 16 bytes of the sender's 32-byte `ActorID`.
- **Payload Checksum (128 bits / 16 Bytes):** Truncated SHA-256 digest `SHA256(Payload)[0..16]`.

---

## 3. Layering Note & Implementation Seam

`[IMPLEMENTATION OBSERVATION]` & `[OPEN QUESTION]`
- In `nex-core/src/transport/types.rs`, transport adapters use a 13-byte physical frame header (`NX` magic 2B + transport tag 2B + flags 1B + length 4B + CRC32 4B).
- In `nex-core/src/transport/socket.rs`, socket sync uses a 4-byte magic `NXSK` followed by length-prefixed bincode payloads.
- In `NEX/00_CONSTITUTION/NEX-WIRE-v1.md`, the 48-byte `NEXW` header defines the formal session/crypto framing.
- **Auditing Note:** The 13-byte link frame encapsulates lower-level physical conduit segments, while the 48-byte `NEXW` header governs the end-to-end cryptographic session layer.
