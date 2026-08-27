import hashlib

def encode_cbor_array(items):
    # Extremely basic and strict CBOR array encoder for the test vector
    # Assumes items are either ints (0-23) or bytes
    out = bytearray()
    out.append(0x80 | len(items)) # Array header (up to 23 items)
    for item in items:
        if isinstance(item, int) and 0 <= item <= 23:
            out.append(item)
        elif isinstance(item, bytes):
            if len(item) <= 23:
                out.append(0x40 | len(item))
                out.extend(item)
            elif len(item) <= 255:
                out.append(0x58)
                out.append(len(item))
                out.extend(item)
            else:
                raise ValueError("Bytes too long for basic encoder")
        else:
            raise ValueError(f"Unsupported type {type(item)}")
    return bytes(out)

def encode_cbor_map(items_dict):
    # Extremely basic and strict CBOR map encoder
    # items_dict keys must be integers 1-23, values can be string, bytes, or pre-encoded bytes
    out = bytearray()
    out.append(0xa0 | len(items_dict)) # Map header
    
    # Deterministic ordering by key (which are ints)
    for k in sorted(items_dict.keys()):
        out.append(k) # key
        v = items_dict[k]
        if isinstance(v, int):
            out.append(v)
        elif isinstance(v, str):
            v_bytes = v.encode('utf-8')
            out.append(0x60 | len(v_bytes))
            out.extend(v_bytes)
        elif isinstance(v, bytes):
            # If it's 32 bytes exactly, it's a byte string
            if len(v) == 32:
                out.append(0x58)
                out.append(32)
                out.extend(v)
            else:
                # Assume it's already encoded CBOR (like a HashRef)
                out.extend(v)
    return bytes(out)


# 1. HashRef Encoding
algorithm_id = 1
digest = bytes.fromhex("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20")
hash_ref_cbor = encode_cbor_array([algorithm_id, digest])
print(f"HashRef CBOR: {hash_ref_cbor.hex()}")

# 2. GenesisRecord Serialization
genesis_map = {
    1: 1, # ProtocolVersion
    2: 1, # ObjectIDAlgorithm
    3: hash_ref_cbor, # CreatorRootDID
    4: "nex.test", # ObjectType
    5: hash_ref_cbor, # InitialPolicyRoot
    6: hash_ref_cbor, # InitialStateRoot
    7: bytes.fromhex("0000000000000000000000000000000000000000000000000000000000000000") # Nonce
}

genesis_cbor = encode_cbor_map(genesis_map)
print(f"GenesisRecord CBOR: {genesis_cbor.hex()}")

# 3. Domain Separated Hashing
domain = b"NEX/OBJECT_ID/v1"
hash_target = domain + genesis_cbor
print(f"Pre-image: {hash_target.hex()}")

object_id = hashlib.sha256(hash_target).digest()
print(f"ObjectID: {object_id.hex()}")
