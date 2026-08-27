use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

pub type ActorID = [u8; 32];
pub type NamespaceID = [u8; 32];
pub type ObjectID = [u8; 32];

pub const OP_REGISTER_LWW: u32 = 0x01;
pub const OP_SET_ADD: u32 = 0x02;
pub const OP_SET_REMOVE: u32 = 0x04;
pub const OP_SEQUENCE_INSERT: u32 = 0x08;
pub const OP_OBJECT_TOMBSTONE: u32 = 0x10;
pub const OP_READ: u32 = 0x01;
pub const OP_WRITE: u32 = 0x02;
pub const OP_ALL: u32 = 0x1F;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    Ed25519 = 1,
    Secp256k1 = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationError {
    SignatureInvalid,
    ExpiredCapability { current_epoch: u64, expires_at: u64 },
    NotYetValid { current_epoch: u64, not_before: u64 },
    RevokedCapability { token_hash: [u8; 32], revocation_epoch: u64 },
    UnauthorizedOperation { requested: u32, allowed: u32 },
    NamespaceMismatch,
    ObjectMismatch,
    RootIssuerMismatch,
    IssuerSubjectMismatch,
    ParentAttenuationViolation(String),
    DelegationDepthExceeded,
    CircularDelegationDetected,
    CyclicDelegationDetected,
    CertificateInvalid,
    InvalidHierarchy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCertificate {
    pub master_actor_id: ActorID,
    pub device_actor_id: ActorID,
    pub not_before_epoch: u64,
    pub expires_at_epoch: u64,
    #[serde(default)]
    pub master_pubkey: Option<Vec<u8>>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub issuer: ActorID,
    pub subject: ActorID,
    pub namespace: NamespaceID,
    pub object_id: Option<ObjectID>,
    pub allowed_operations: u32,
    pub delegation_depth: u8,
    pub not_before_epoch: u64,
    pub expires_at_epoch: u64,
    pub parent_token_hash: Option<[u8; 32]>,
}

impl CapabilityToken {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.issuer);
        buf.extend_from_slice(&self.subject);
        buf.extend_from_slice(&self.namespace);
        if let Some(obj) = &self.object_id {
            buf.push(1);
            buf.extend_from_slice(obj);
        } else {
            buf.push(0);
        }
        buf.extend_from_slice(&self.allowed_operations.to_le_bytes());
        buf.push(self.delegation_depth);
        buf.extend_from_slice(&self.not_before_epoch.to_le_bytes());
        buf.extend_from_slice(&self.expires_at_epoch.to_le_bytes());
        if let Some(p) = &self.parent_token_hash {
            buf.push(1);
            buf.extend_from_slice(p);
        } else {
            buf.push(0);
        }
        buf
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/CAPABILITY_TOKEN/v1");
        hasher.update(&self.canonical_bytes());
        hasher.finalize().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityProof {
    pub token: CapabilityToken,
    pub issuer_pubkey: Option<Vec<u8>>,
    pub parent_proof: Option<Box<CapabilityProof>>,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationEpochFence {
    pub issuer: ActorID,
    pub target_subject: ActorID,
    pub target_namespace: Option<NamespaceID>,
    pub revoked_at_epoch: u64,
    pub reason: String,
    pub issuer_pubkey: Option<Vec<u8>>,
    pub signature: Vec<u8>,
}
