package core

// AuthorityEvaluator handles Capability resolution, Delegation chains, and Revocation state.
type AuthorityEvaluator interface {
	// VerifyCapability traces the author's device key to a Root DID and ensures the CapabilityRef is valid and unrevoked.
	VerifyCapability(mutation MutationEnvelope, objectState ObjectState) (bool, error)
}

// CausalEvaluator handles DAG traversal, missing history detection, and conflict resolution.
type CausalEvaluator interface {
	Evaluate(mutation MutationEnvelope, localHeads [][32]byte) (bool, error)
}

// CryptoSuite abstracts cryptographic operations to prevent algorithm lock-in.
type CryptoSuite interface {
	VerifySignature(pubKey []byte, payload []byte, signature []byte) bool
	Hash(payload []byte) [32]byte
}

// ResourcePolicy enforces local sovereignty, determining the cost of requested operations.
type ResourcePolicy interface {
	// CheckBounds evaluates if processing the given Envelope and its potential historical dependencies exceeds local limits.
	CheckBounds(mutation MutationEnvelope) (bool, error)
}

// TransportAdapter multiplexes various ProtocolMessages over a Sync Session.
type TransportAdapter interface {
	BroadcastMutation(mutation MutationEnvelope) error
	RequestHistory(targetHeads [][32]byte) error
	ReceiveMessage() (ProtocolMessage, error)
}

// ProtocolMessage represents the multiplexed transport layer.
type MessageType uint8
const (
	MessageTypeSyncNegotiationReq MessageType = 0x10
	MessageTypeMutationPush       MessageType = 0x20
	MessageTypeHistoryRequest     MessageType = 0x30
)

type ProtocolMessage struct {
	MessageType MessageType
	Payload     []byte
}

// ObjectState is the lightweight metadata and capability graph (omitting bulk content).
type ObjectState struct {
	// Implementation details defined by Object Architecture
}
