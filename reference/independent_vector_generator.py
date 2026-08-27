import json
import hashlib
from normative_model import NexNormativeModel

def generate_vectors():
    suite = {
        "version": "nex-protocol-v1.0-final-independent",
        "description": "Final independent vectors testing genesis collision determinism, identity authority, and strict schema closure.",
        "vectors": []
    }

    # 1. Genesis Collision Determinism
    id1 = b"82015820ffffff0000000000000000000000000000000000000000000000000000000000"
    id2 = b"820158200000000000000000000000000000000000000000000000000000000000000000"
    winner = NexNormativeModel.resolve_genesis_collision([id1, id2])
    suite["vectors"].append({
        "category": "Genesis",
        "test_name": "Genesis_Collision_Determinism_Min_Lex",
        "inputs": {"candidates": [id1.hex(), id2.hex()]},
        "expected_winner_hex": winner.hex(),
        "expected_loser_state": "GENESIS_COLLISION"
    })

    # 2. Identity Genesis Authority Invariant
    root_key = b'\x01'*32
    author_key = b'\x01'*32
    desc = NexNormativeModel.construct_genesis_descriptor(1, "NEX", 1, root_key)
    auth_valid = NexNormativeModel.validate_identity_genesis_authority(author_key, desc, True, True)
    suite["vectors"].append({
        "category": "Identity",
        "test_name": "Identity_Genesis_Valid_Authority",
        "description": "Validates AuthorDeviceKey == RootDeviceKey with nil contexts.",
        "inputs": {"author_key": author_key.hex(), "root_key": root_key.hex(), "ctx_is_nil": True, "cap_is_nil": True},
        "expected_authority_state": auth_valid
    })
    
    # 3. Identity Genesis Authority Invariant (Failure)
    author_key_invalid = b'\x02'*32
    auth_invalid = NexNormativeModel.validate_identity_genesis_authority(author_key_invalid, desc, True, True)
    suite["vectors"].append({
        "category": "Identity",
        "test_name": "Identity_Genesis_Invalid_Authority",
        "description": "Fails when AuthorDeviceKey != RootDeviceKey.",
        "inputs": {"author_key": author_key_invalid.hex(), "root_key": root_key.hex(), "ctx_is_nil": True, "cap_is_nil": True},
        "expected_authority_state": auth_invalid
    })

    # 4. Strict Genesis Descriptor Construction
    desc_bytes = NexNormativeModel.nex_cbor_encode(desc)
    obj_id = NexNormativeModel.derive_object_id(desc)
    suite["vectors"].append({
        "category": "Genesis",
        "test_name": "Strict_Genesis_Descriptor_Encoding",
        "description": "Encodes the strictly typed integer CBOR map for GenesisDescriptor.",
        "expected_descriptor_hex": desc_bytes.hex(),
        "expected_object_id_hex": obj_id.hex()
    })

    with open("conformance_suite_v1.0_final_independent.json", "w") as f:
        json.dump(suite, f, indent=4)
    print("Successfully generated conformance_suite_v1.0_final_independent.json")

if __name__ == "__main__":
    generate_vectors()
