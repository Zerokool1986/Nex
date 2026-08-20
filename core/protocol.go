package core

// MutationType identifies the specific state transition class.
type MutationType uint8

const (
	MutationTypeStateUpdate          MutationType = 0x01
	MutationTypeCapabilityDelegation MutationType = 0x02
	MutationTypeCapabilityRevocation MutationType = 0x03
	MutationTypeTombstone            MutationType = 0x04
	MutationTypeResurrection         MutationType = 0x05
	MutationTypeCheckpoint           MutationType = 0x06
)

// MutationEnvelope is the explicitly typed, causally linked payload.
type MutationEnvelope struct {
	ObjectID        ObjectID     `cbor:"object_id"`
	MutationType    MutationType `cbor:"mutation_type"`
	AuthorDeviceKey []byte       `cbor:"author_device_key"` // Ed25519 PubKey
	CausalParents   [][32]byte   `cbor:"causal_parents"`    // Array of MutationIDs
	CapabilityRef   [32]byte     `cbor:"capability_ref"`    // MutationID granting authority
	Payload         []byte       `cbor:"payload"`           // Type-specific CBOR
	Signature       []byte       `cbor:"signature"`
}

// Disposition represents the singular local resource/validity determination.
type Disposition int

const (
	DispositionAccepted Disposition = iota
	DispositionQuarantined
	DispositionRejected
)

// Condition represents orthogonal causal or trust conditions.
// Evaluated as a bitmask/set.
type Condition uint32

const (
	ConditionConflict         Condition = 1 << 0
	ConditionNeedsHistory     Condition = 1 << 1
	ConditionNeedsTrustAnchor Condition = 1 << 2
)

// EvaluationResult explicitly separates Dispositions from Conditions.
type EvaluationResult struct {
	Disposition Disposition
	Conditions  Condition // Bitmask of Condition flags
	Message     string
}

// CheckpointPayload defines the verifiable state boundary commitments.
type CheckpointPayload struct {
	CausalHeads      [][32]byte `cbor:"causal_heads"`
	StateCommitment  []byte     `cbor:"state_commitment"`
	AuthorityState   []byte     `cbor:"authority_state"`
	CoverageWindow   uint64     `cbor:"coverage_window"`
}
