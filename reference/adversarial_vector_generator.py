import json

def generate_adversarial_vectors():
    suite = {
        "version": "nex-protocol-v1.0-adversarial",
        "description": "Constructed adversarial byte sequences to aggressively test CBOR, COSE, and rejection boundaries.",
        "vectors": []
    }

    # 1. CBOR encoding violations
    # - Duplicate keys in map: {1: 10, 1: 20}
    # Raw CBOR: 0xA2 (map of 2 pairs) 0x01 (int 1) 0x0A (int 10) 0x01 (int 1) 0x14 (int 20)
    cbor_duplicate_keys = bytes([0xA2, 0x01, 0x0A, 0x01, 0x14])
    suite["vectors"].append({
        "category": "CBOR",
        "test_name": "Reject_Duplicate_Keys",
        "description": "CBOR map containing duplicate integer key 1.",
        "inputs": {"raw_bytes": cbor_duplicate_keys.hex()},
        "expected_error": "INVALID_SCHEMA" # Or INVALID_CBOR if defined
    })

    # - Float tag in CBOR (Float 1.0 -> 0xFA 0x3F 0x80 0x00 0x00)
    cbor_float = bytes([0xA1, 0x01, 0xFA, 0x3F, 0x80, 0x00, 0x00])
    suite["vectors"].append({
        "category": "CBOR",
        "test_name": "Reject_Float",
        "description": "CBOR map containing a floating point value.",
        "inputs": {"raw_bytes": cbor_float.hex()},
        "expected_error": "INVALID_SCHEMA"
    })

    # 2. HashRef Violations
    # HashRef [AlgorithmID 1, Digest: 31 bytes (too short)]
    # CBOR: 0x82 (array of 2) 0x01 (int 1) 0x5F (bytes indefinite, wait no, 31 bytes is 0x58 0x1F) -> let's use definite
    # 31 byte string: 0x58 0x1F + 31 zero bytes
    cbor_hash_short = bytes([0x82, 0x01, 0x58, 0x1F]) + (b'\x00' * 31)
    suite["vectors"].append({
        "category": "HashRef",
        "test_name": "HashRef_Short_Digest",
        "description": "SHA-256 HashRef with exactly 31 bytes.",
        "inputs": {"raw_bytes": cbor_hash_short.hex()},
        "expected_error": "INVALID_HASH_REF"
    })
    
    # HashRef unknown algorithm (AlgorithmID = 2)
    # CBOR: 0x82 0x02 0x58 0x20 + 32 zero bytes
    cbor_hash_unknown_alg = bytes([0x82, 0x02, 0x58, 0x20]) + (b'\x00' * 32)
    suite["vectors"].append({
        "category": "HashRef",
        "test_name": "HashRef_Unknown_Alg",
        "description": "HashRef with AlgorithmID 2.",
        "inputs": {"raw_bytes": cbor_hash_unknown_alg.hex()},
        "expected_error": "INVALID_HASH_REF"
    })

    with open("conformance_suite_v1.0_adversarial.json", "w") as f:
        json.dump(suite, f, indent=4)
    print("Adversarial vector generator successfully generated conformance_suite_v1.0_adversarial.json")

if __name__ == "__main__":
    generate_adversarial_vectors()
