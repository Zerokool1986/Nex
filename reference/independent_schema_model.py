import json
import hashlib

class IndependentSchemaModel:
    """
    An independent, type-strict implementation of Nex Wire Spec v0.9.
    Does not import or share code with oracle.py.
    """
    
    @staticmethod
    def encode_hash_ref(alg_id: int, digest_hex: str) -> str:
        # Array of 2 elements: [uint, bstr]
        # 82 = Array(2)
        # 01 = uint(1) for example
        # 58 20 = bstr(32) followed by 32 bytes
        alg_cbor = f"{alg_id:02x}"
        digest_bytes = bytes.fromhex(digest_hex)
        bstr_header = f"58{len(digest_bytes):02x}"
        return f"82{alg_cbor}{bstr_header}{digest_hex}"

    @staticmethod
    def build_sig_structure(sig_alg_id: int, mutation_body_cbor: str) -> str:
        # ["Signature1", bstr({1: sig_alg_id}), h'', bstr(MutationBody)]
        # Protected header: A1 01 <sig_alg_id> -> A10101 (if sig_alg_id = 1)
        protected_header_cbor = f"a101{sig_alg_id:02x}"
        protected_header_bstr = f"43{protected_header_cbor}" # bstr of length 3
        
        # External AAD: h'' -> 40
        external_aad = "40"
        
        # Payload: bstr(MutationBody)
        mb_len = len(bytes.fromhex(mutation_body_cbor))
        payload_bstr_header = "59" + f"{mb_len:04x}" if mb_len > 23 else f"{64+mb_len:02x}"
        
        # 84 = Array(4)
        # 6a = tstr(10) "Signature1" -> 5369676e617475726531
        return f"846a5369676e617475726531{protected_header_bstr}{external_aad}{payload_bstr_header}{mutation_body_cbor}"

def generate_independent_vectors():
    vectors = []
    
    # Vector 1: HashRef Generation
    vectors.append({
        "type": "HashRef",
        "inputs": {"alg_id": 1, "digest": "00"*32},
        "expected_cbor_hex": IndependentSchemaModel.encode_hash_ref(1, "00"*32)
    })
    
    # Vector 2: Sig_structure Generation
    vectors.append({
        "type": "Sig_structure",
        "inputs": {"sig_alg_id": 1, "mutation_body": "a0"}, # empty map
        "expected_cbor_hex": IndependentSchemaModel.build_sig_structure(1, "a0")
    })

    with open("independent_vectors_v0.9.json", "w") as f:
        json.dump(vectors, f, indent=4)
    print("Independent vectors generated.")

if __name__ == "__main__":
    generate_independent_vectors()
