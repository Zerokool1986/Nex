import hashlib

class NexNormativeModel:
    """
    Independent Normative Model for Nex Protocol v1.0.
    Executes the purely constructible identifier graph and determinism logic.
    """

    @staticmethod
    def nex_cbor_encode(data) -> bytes:
        if data is None: return bytes([0xf6])
        elif isinstance(data, int):
            if data < 24: return bytes([data])
            elif data < 256: return bytes([0x18, data])
            else: raise ValueError("Int too large")
        elif isinstance(data, bytes):
            if len(data) < 24: return bytes([0x40 | len(data)]) + data
            else: return bytes([0x58, len(data)]) + data
        elif isinstance(data, str):
            encoded = data.encode('utf-8')
            if len(encoded) < 24: return bytes([0x60 | len(encoded)]) + encoded
            else: return bytes([0x78, len(encoded)]) + encoded
        elif isinstance(data, list):
            res = bytes([0x80 | len(data)])
            for item in data: res += NexNormativeModel.nex_cbor_encode(item)
            return res
        elif isinstance(data, dict):
            encoded_pairs = []
            for k, v in data.items():
                encoded_pairs.append((NexNormativeModel.nex_cbor_encode(k), NexNormativeModel.nex_cbor_encode(v)))
            encoded_pairs.sort(key=lambda x: x[0])
            res = bytes([0xA0 | len(encoded_pairs)])
            for ek, ev in encoded_pairs: res += ek + ev
            return res
        raise TypeError("Type not permitted")

    @staticmethod
    def create_hash_ref(alg_id: int, digest: bytes) -> bytes:
        return NexNormativeModel.nex_cbor_encode([alg_id, digest])

    @staticmethod
    def construct_genesis_descriptor(obj_type: int, domain: str, version: int, root_key: bytes = None, creator_nonce: bytes = None) -> dict:
        if creator_nonce is None or len(creator_nonce) < 16 or len(creator_nonce) > 32:
            raise ValueError("INVALID_SCHEMA: CreatorNonce must be 16-32 bytes.")
        desc = {1: obj_type, 3: domain, 4: version, 5: creator_nonce}
        if obj_type == 1:
            if root_key is None or len(root_key) != 32:
                raise ValueError("INVALID_SCHEMA")
            desc[2] = root_key
        return desc

    @staticmethod
    def derive_object_id(genesis_descriptor: dict) -> bytes:
        desc_bytes = NexNormativeModel.nex_cbor_encode(genesis_descriptor)
        digest = hashlib.sha256(b"NEX/OBJECT_ID/v1" + desc_bytes).digest()
        return NexNormativeModel.create_hash_ref(1, digest)

    @staticmethod
    def resolve_genesis_collision(mutation_ids: list[bytes]) -> bytes:
        """Deterministically selects GenesisWinner = min_lex(MutationID)"""
        if not mutation_ids:
            return b""
        return min(mutation_ids)

    @staticmethod
    def validate_identity_genesis_authority(author_key: bytes, genesis_descriptor: dict, ctx_is_nil: bool, cap_is_nil: bool) -> str:
        if genesis_descriptor.get(1) != 1:
            return "NOT_IDENTITY_OBJECT"
        root_key = genesis_descriptor.get(2)
        if author_key == root_key and ctx_is_nil and cap_is_nil:
            return "VALID"
        return "INVALID_AUTHORITY"

if __name__ == "__main__":
    print("Executing Final Normative Model Invariants:")
    
    # 1. Strict Genesis Descriptor & ObjectID
    root_key = b'\x00'*32
    nonce = b'\xAA'*16
    desc = NexNormativeModel.construct_genesis_descriptor(1, "NEX", 1, root_key, nonce)
    obj_id = NexNormativeModel.derive_object_id(desc)
    print(f"Constructed ObjectID (Identity): {obj_id.hex()}")
    
    # 2. Identity Genesis Authority Equation
    auth = NexNormativeModel.validate_identity_genesis_authority(root_key, desc, True, True)
    print(f"Identity Authority Invariant: {auth}")
    
    # 3. Genesis Collision Determinism
    id1 = b"82015820ffffff..."
    id2 = b"82015820000000..."
    winner = NexNormativeModel.resolve_genesis_collision([id1, id2])
    print(f"Genesis Collision Winner (min_lex): {winner.decode('utf-8')}")
