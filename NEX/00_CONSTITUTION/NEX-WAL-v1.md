# NEX/WAL/v1: Write-Ahead Log Specification

**Status:** STRICTLY FROZEN & IMMUTABLE

## Magic Header
Every valid WAL begins with an 8-byte ASCII header: `NEXWAL01` (0x4E455857414C3031).

## Record Framing
Each mutation record contains:
1. `record_length: u32` (Big-Endian)
2. `record_crc32: u32` (CRC-32 checksum of payload)
3. `mutation_payload: [u8; record_length]` (Deterministic CBOR encoded mutation)
