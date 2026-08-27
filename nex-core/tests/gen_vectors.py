import hashlib
import json
import zlib
import struct

def crc32_payload(payload: bytes) -> int:
    return zlib.crc32(payload) & 0xFFFFFFFF

k0 = hashlib.sha256(b"NEX/ACTOR_ID/v1" + bytes([1]) + b"\x00" * 32).hexdigest()
k1 = hashlib.sha256(b"NEX/ACTOR_ID/v1" + bytes([1]) + b"\x01" * 32).hexdigest()
smt0 = hashlib.sha256(b"NEX/SMT_KEY/v1" + b"\x00" * 32).hexdigest()

p1 = bytes.fromhex("01020304")
crc1 = crc32_payload(p1)
frame1 = struct.pack(">2sHBI", b"NX", 2, 0, len(p1)) + struct.pack(">I", crc1) + p1

p2 = b""
crc2 = crc32_payload(p2)
frame2 = struct.pack(">2sHBI", b"NX", 1, 0, len(p2)) + struct.pack(">I", crc2) + p2

data = {
  "$schema": "https://nex.org/schemas/conformance_v1.json",
  "version": "1.0.0",
  "description": "Language-independent golden conformance vectors for NEX protocol verification",
  "identity_vectors": [
    {
      "description": "Ed25519 ActorID derivation from 32-byte zero key",
      "key_type": 1,
      "public_key_hex": ("00" * 32),
      "domain_separator": "NEX/ACTOR_ID/v1",
      "expected_actor_id_hex": k0
    },
    {
      "description": "Ed25519 ActorID derivation from incremental key",
      "key_type": 1,
      "public_key_hex": ("01" * 32),
      "domain_separator": "NEX/ACTOR_ID/v1",
      "expected_actor_id_hex": k1
    }
  ],
  "wire_frame_vectors": [
    {
      "description": "13-byte wire frame for 4-byte payload 0x01020304 over QUIC (0x02)",
      "transport_tag": 2,
      "flags": 0,
      "payload_hex": "01020304",
      "expected_frame_hex": frame1.hex(),
      "expected_crc32_hex": f"{crc1:08x}"
    },
    {
      "description": "13-byte wire frame for empty payload over Mesh (0x01)",
      "transport_tag": 1,
      "flags": 0,
      "payload_hex": "",
      "expected_frame_hex": frame2.hex(),
      "expected_crc32_hex": f"{crc2:08x}"
    }
  ],
  "smt_key_vectors": [
    {
      "description": "SMT Key hash for MutationID 0x00..00",
      "mutation_id_hex": "00" * 32,
      "domain_separator": "NEX/SMT_KEY/v1",
      "expected_smt_key_hex": smt0
    }
  ],
  "wal_record_vectors": [
    {
      "description": "WAL file header (8 Bytes)",
      "expected_header_hex": "4e45585701000000"
    }
  ]
}

with open("/mnt/c/Users/Admin/.gemini/antigravity/brain/1000b28c-b231-4f08-bfa4-ae8b1c1eec73/scratch/nex-r13-benchmark/crates/nex-core/tests/golden_vectors_r30.json", "w") as f:
    json.dump(data, f, indent=2)
print("Updated golden vectors successfully.")
