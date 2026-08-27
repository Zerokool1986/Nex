package core

// RootIdentity represents the Sovereign DID of a user.
// It acts as the root of trust for an IdentityObject graph.
type RootIdentity struct {
	DID string // e.g., "did:nex:..."
}

// ObjectID is the self-certifying hash of the GenesisRecord.
type ObjectID [32]byte

// GenesisRecord is the deterministic payload used to derive the ObjectID.
// Must be serialized using Strict Deterministic CBOR.
type GenesisRecord struct {
	CreatorRootDID    string   `cbor:"creator_root_did"`
	ObjectType        string   `cbor:"object_type"`
	InitialPolicyRoot []byte   `cbor:"initial_policy_root"`
	InitialStateRoot  []byte   `cbor:"initial_state_root"`
	Nonce             [32]byte `cbor:"nonce"`
	Timestamp         uint64   `cbor:"timestamp"`
}

// ComputeObjectID mathematically derives the immutable ObjectID from the Genesis payload
// using explicit cryptographic domain separation.
func (g *GenesisRecord) ComputeObjectID(crypto CryptoSuite) (ObjectID, error) {
	// Canonicalize to CBOR
	cborBytes, err := CanonicalCBOR(g)
	if err != nil {
		return ObjectID{}, err
	}

	// Apply Domain Separation
	domainPrefix := []byte("NEX/OBJECT_ID/v1")
	hashTarget := append(domainPrefix, cborBytes...)

	// Hash
	hashBytes := crypto.Hash(hashTarget)
	var id ObjectID
	copy(id[:], hashBytes)
	return id, nil
}

// CanonicalCBOR is a placeholder for a strict RFC 8949 deterministic CBOR encoder.
func CanonicalCBOR(v interface{}) ([]byte, error) {
	// TODO: Implement deterministic CBOR encoding
	return []byte{}, nil
}
