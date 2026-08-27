use std::collections::{BTreeMap, HashSet};
use sha2::{Sha256, Digest};
use crate::identity::types::{
    ActorID, KeyType, CapabilityToken, CapabilityProof, DeviceCertificate, AuthorizationError
};
use crate::object::types::{NamespaceID, ObjectID};

pub const DOMAIN_ACTOR_ID: &[u8] = b"NEX/ACTOR_ID/v1";
pub const DOMAIN_CAPABILITY_TOKEN: &[u8] = b"NEX/CAPABILITY_TOKEN/v1";
pub const DOMAIN_DEVICE_CERT: &[u8] = b"NEX/DEVICE_CERT/v1";

pub fn derive_actor_id(key_type: KeyType, public_key: &[u8]) -> ActorID {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_ACTOR_ID);
    hasher.update(&[key_type as u8]);
    hasher.update(public_key);
    hasher.finalize().into()
}

pub fn hash_capability_token(token: &CapabilityToken) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_CAPABILITY_TOKEN);
    hasher.update(&token.issuer);
    hasher.update(&token.subject);
    hasher.update(&token.namespace);
    if let Some(obj) = &token.object_id {
        hasher.update(&[1u8]);
        hasher.update(obj);
    } else {
        hasher.update(&[0u8]);
    }
    hasher.update(&token.allowed_operations.to_le_bytes());
    hasher.update(&[token.delegation_depth]);
    hasher.update(&token.not_before_epoch.to_le_bytes());
    hasher.update(&token.expires_at_epoch.to_le_bytes());
    if let Some(parent) = &token.parent_token_hash {
        hasher.update(&[1u8]);
        hasher.update(parent);
    } else {
        hasher.update(&[0u8]);
    }
    hasher.finalize().into()
}

pub fn verify_capability_chain(
    proof: &CapabilityProof,
    requested_op: u32,
    target_namespace: &NamespaceID,
    target_object: Option<&ObjectID>,
    current_epoch: u64,
    active_revocations: &BTreeMap<[u8; 32], u64>,
    root_authority: &ActorID,
) -> Result<ActorID, AuthorizationError> {
    let mut visited_tokens: HashSet<[u8; 32]> = HashSet::new();
    verify_capability_recursive(
        proof,
        requested_op,
        target_namespace,
        target_object,
        current_epoch,
        active_revocations,
        root_authority,
        &mut visited_tokens,
    )
}

fn verify_capability_recursive(
    proof: &CapabilityProof,
    requested_op: u32,
    target_namespace: &NamespaceID,
    target_object: Option<&ObjectID>,
    current_epoch: u64,
    active_revocations: &BTreeMap<[u8; 32], u64>,
    root_authority: &ActorID,
    visited_tokens: &mut HashSet<[u8; 32]>,
) -> Result<ActorID, AuthorizationError> {
    let token = &proof.token;
    let token_hash = hash_capability_token(token);

    // --- 1. Acyclic Delegation Check ---
    if !visited_tokens.insert(token_hash) {
        return Err(AuthorizationError::CyclicDelegationDetected);
    }

    // --- 2. Temporal Validity ---
    if current_epoch < token.not_before_epoch {
        return Err(AuthorizationError::NotYetValid {
            current_epoch,
            not_before: token.not_before_epoch,
        });
    }
    if current_epoch > token.expires_at_epoch {
        return Err(AuthorizationError::ExpiredCapability {
            current_epoch,
            expires_at: token.expires_at_epoch,
        });
    }

    // --- 3. Revocation Check ---
    if let Some(&revocation_epoch) = active_revocations.get(&token_hash) {
        if current_epoch >= revocation_epoch {
            return Err(AuthorizationError::RevokedCapability {
                token_hash,
                revocation_epoch,
            });
        }
    }

    // --- 4. Operation Scope ---
    if (token.allowed_operations & requested_op) != requested_op {
        return Err(AuthorizationError::UnauthorizedOperation {
            requested: requested_op,
            allowed: token.allowed_operations,
        });
    }

    // --- 5. Namespace Scope ---
    if token.namespace != *target_namespace {
        return Err(AuthorizationError::NamespaceMismatch);
    }

    // --- 6. Object Scope ---
    if let Some(token_obj) = &token.object_id {
        if target_object != Some(token_obj) {
            return Err(AuthorizationError::ObjectMismatch);
        }
    }

    // --- 7. Signature Validity (Ed25519 Curve Verification) ---
    if proof.signature.is_empty() {
        return Err(AuthorizationError::SignatureInvalid);
    }
    if let Some(pubkey_bytes) = &proof.issuer_pubkey {
        if pubkey_bytes.len() != 32 || proof.signature.len() != 64 {
            return Err(AuthorizationError::SignatureInvalid);
        }
        let expected_actor = derive_actor_id(KeyType::Ed25519, pubkey_bytes);
        if expected_actor != token.issuer {
            return Err(AuthorizationError::RootIssuerMismatch);
        }
        let mut pk_arr = [0u8; 32];
        pk_arr.copy_from_slice(pubkey_bytes);
        let vk = match ed25519_dalek::VerifyingKey::from_bytes(&pk_arr) {
            Ok(k) => k,
            Err(_) => return Err(AuthorizationError::SignatureInvalid),
        };
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&proof.signature);
        let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);
        if vk.verify_strict(&token_hash, &sig).is_err() {
            return Err(AuthorizationError::SignatureInvalid);
        }
    }

    // --- 8. Delegation Chaining & Attenuation ---
    match (&token.parent_token_hash, &proof.parent_proof) {
        (None, None) => {
            // Root Capability: Issuer must be the Root Authority
            if token.issuer != *root_authority {
                return Err(AuthorizationError::RootIssuerMismatch);
            }
            Ok(token.subject)
        }
        (Some(parent_hash), Some(parent_proof)) => {
            let actual_parent_hash = hash_capability_token(&parent_proof.token);
            if *parent_hash != actual_parent_hash {
                return Err(AuthorizationError::ParentAttenuationViolation("Parent token hash mismatch".into()));
            }

            // Issuer must be the subject of the parent capability
            if token.issuer != parent_proof.token.subject {
                return Err(AuthorizationError::IssuerSubjectMismatch);
            }

            // Attenuation Invariant 1: Operation Subsumption
            if (token.allowed_operations & !parent_proof.token.allowed_operations) != 0 {
                return Err(AuthorizationError::ParentAttenuationViolation("Privilege escalation: operation not in parent".into()));
            }

            // Attenuation Invariant 2: Namespace Bounding
            if token.namespace != parent_proof.token.namespace {
                return Err(AuthorizationError::ParentAttenuationViolation("Namespace widening violation".into()));
            }

            // Attenuation Invariant 3: Object Bounding
            if let Some(parent_obj) = &parent_proof.token.object_id {
                if token.object_id.as_ref() != Some(parent_obj) {
                    return Err(AuthorizationError::ParentAttenuationViolation("Object scope widening violation".into()));
                }
            }

            // Attenuation Invariant 4: Temporal Bounding
            if token.not_before_epoch < parent_proof.token.not_before_epoch ||
               token.expires_at_epoch > parent_proof.token.expires_at_epoch {
                return Err(AuthorizationError::ParentAttenuationViolation("Temporal window exceeds parent".into()));
            }

            // Attenuation Invariant 5: Depth Monotonicity
            if parent_proof.token.delegation_depth == 0 {
                return Err(AuthorizationError::DelegationDepthExceeded);
            }
            if token.delegation_depth >= parent_proof.token.delegation_depth {
                return Err(AuthorizationError::ParentAttenuationViolation("Delegation depth must strictly decrement".into()));
            }

            // Recursively verify parent capability
            verify_capability_recursive(
                parent_proof,
                parent_proof.token.allowed_operations,
                &parent_proof.token.namespace,
                parent_proof.token.object_id.as_ref(),
                current_epoch,
                active_revocations,
                root_authority,
                visited_tokens,
            )?;

            Ok(token.subject)
        }
        _ => Err(AuthorizationError::ParentAttenuationViolation("Mismatched parent token hash and parent proof".into())),
    }
}

pub fn verify_device_certificate(
    cert: &DeviceCertificate,
    master_actor: &ActorID,
    current_epoch: u64,
) -> Result<(), AuthorizationError> {
    if cert.master_actor_id != *master_actor {
        return Err(AuthorizationError::RootIssuerMismatch);
    }
    if current_epoch < cert.not_before_epoch {
        return Err(AuthorizationError::NotYetValid {
            current_epoch,
            not_before: cert.not_before_epoch,
        });
    }
    if current_epoch > cert.expires_at_epoch {
        return Err(AuthorizationError::ExpiredCapability {
            current_epoch,
            expires_at: cert.expires_at_epoch,
        });
    }
    if cert.signature.is_empty() {
        return Err(AuthorizationError::SignatureInvalid);
    }
    if let Some(pubkey_bytes) = &cert.master_pubkey {
        if pubkey_bytes.len() == 32 && cert.signature.len() == 64 {
            let expected_actor = derive_actor_id(KeyType::Ed25519, pubkey_bytes);
            if expected_actor != cert.master_actor_id {
                return Err(AuthorizationError::RootIssuerMismatch);
            }
            let mut pk_arr = [0u8; 32];
            pk_arr.copy_from_slice(pubkey_bytes);
            if let Ok(vk) = ed25519_dalek::VerifyingKey::from_bytes(&pk_arr) {
                let mut sig_arr = [0u8; 64];
                sig_arr.copy_from_slice(&cert.signature);
                let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);
                
                let mut hasher = Sha256::new();
                hasher.update(DOMAIN_DEVICE_CERT);
                hasher.update(&cert.master_actor_id);
                hasher.update(&cert.device_actor_id);
                hasher.update(&cert.not_before_epoch.to_le_bytes());
                hasher.update(&cert.expires_at_epoch.to_le_bytes());
                let cert_hash: [u8; 32] = hasher.finalize().into();
                
                if vk.verify_strict(&cert_hash, &sig).is_err() {
                    return Err(AuthorizationError::SignatureInvalid);
                }
            }
        }
    }
    Ok(())
}

pub fn verify_device_certificate_with_crl(
    cert: &DeviceCertificate,
    master_actor: &ActorID,
    current_epoch: u64,
    revoked_devices: &std::collections::BTreeSet<ActorID>,
) -> Result<(), AuthorizationError> {
    if revoked_devices.contains(&cert.device_actor_id) {
        return Err(AuthorizationError::CertificateInvalid);
    }
    verify_device_certificate(cert, master_actor, current_epoch)
}

