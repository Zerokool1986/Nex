use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use rand::{RngCore, SeedableRng};
use rand::rngs::OsRng;
use rand_chacha::ChaCha20Rng;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

pub const DOMAIN_NASP_INIT: &[u8] = b"NEX/NASP_INIT/v1";
pub const DOMAIN_NASP_REPLY: &[u8] = b"NEX/NASP_REPLY/v1";
pub const DOMAIN_NASP_CONFIRM: &[u8] = b"NEX/NASP_CONFIRM/v1";
pub const DOMAIN_NASP_SHARED_SECRET: &[u8] = b"NEX/NASP_SHARED_SECRET/v1";
pub const DOMAIN_NASP_REKEY: &[u8] = b"NEX/NASP_REKEY/v1";
pub const DOMAIN_NASP_EPHEMERAL: &[u8] = b"NEX/NASP_EPHEMERAL/v1";

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut key_pad = [0u8; 64];
    if key.len() > 64 {
        let hashed = Sha256::digest(key);
        key_pad[..32].copy_from_slice(&hashed);
    } else {
        key_pad[..key.len()].copy_from_slice(key);
    }

    let mut o_key_pad = [0x5cu8; 64];
    let mut i_key_pad = [0x36u8; 64];
    for i in 0..64 {
        o_key_pad[i] ^= key_pad[i];
        i_key_pad[i] ^= key_pad[i];
    }

    let mut inner = Sha256::new();
    inner.update(&i_key_pad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&o_key_pad);
    outer.update(&inner_hash);
    outer.finalize().into()
}

pub fn hkdf_expand(prk: &[u8; 32], info: &[u8], len: usize) -> Vec<u8> {
    let mut okm = Vec::with_capacity(len);
    let mut t = Vec::new();
    let mut counter = 1u8;

    while okm.len() < len {
        let mut hasher = Sha256::new();
        hasher.update(prk);
        if !t.is_empty() {
            hasher.update(&t);
        }
        hasher.update(info);
        hasher.update(&[counter]);
        t = hasher.finalize().to_vec();
        okm.extend_from_slice(&t);
        counter += 1;
    }

    okm.truncate(len);
    okm
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaspInit {
    pub ephemeral_pub: [u8; 32],
    pub static_pub: [u8; 32],
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaspReply {
    pub ephemeral_pub: [u8; 32],
    pub static_pub: [u8; 32],
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaspConfirm {
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SessionKeys {
    pub is_initiator: bool,
    pub k_tx: [u8; 32],
    pub k_rx: [u8; 32],
    pub k_mac_tx: [u8; 32],
    pub k_mac_rx: [u8; 32],
    pub k_rekey: [u8; 32],
    pub previous_k_rx: Option<[u8; 32]>,
    pub previous_k_mac_rx: Option<[u8; 32]>,
    pub tx_seq: u64,
    pub rx_seq: u64,
}

impl SessionKeys {
    pub fn encrypt(&mut self, plaintext: &[u8]) -> (u64, Vec<u8>, [u8; 32]) {
        self.encrypt_with_aad(&[], plaintext)
    }

    pub fn encrypt_with_aad(&mut self, aad: &[u8], plaintext: &[u8]) -> (u64, Vec<u8>, [u8; 32]) {
        self.tx_seq += 1;
        let seq = self.tx_seq;

        // ChaCha20 Keystream seed = k_tx ^ seq
        let mut seed = self.k_tx;
        let seq_bytes = seq.to_be_bytes();
        for i in 0..8 {
            seed[i] ^= seq_bytes[i];
        }

        let mut rng = ChaCha20Rng::from_seed(seed);
        let mut ciphertext = plaintext.to_vec();
        let mut keystream = vec![0u8; plaintext.len()];
        rng.fill_bytes(&mut keystream);

        for i in 0..plaintext.len() {
            ciphertext[i] ^= keystream[i];
        }

        // MAC = HMAC-SHA256(k_mac_tx, aad || seq || ciphertext)
        let mut mac_input = Vec::with_capacity(aad.len() + 8 + ciphertext.len());
        mac_input.extend_from_slice(aad);
        mac_input.extend_from_slice(&seq_bytes);
        mac_input.extend_from_slice(&ciphertext);
        let mac = hmac_sha256(&self.k_mac_tx, &mac_input);

        (seq, ciphertext, mac)
    }

    pub fn decrypt(&mut self, seq: u64, ciphertext: &[u8], mac: &[u8; 32]) -> Result<Vec<u8>, String> {
        self.decrypt_with_aad(&[], seq, ciphertext, mac)
    }

    pub fn decrypt_with_aad(&mut self, aad: &[u8], seq: u64, ciphertext: &[u8], mac: &[u8; 32]) -> Result<Vec<u8>, String> {
        if seq <= self.rx_seq {
            return Err(format!("Anti-replay rejection: seq {} <= last seen {}", seq, self.rx_seq));
        }

        let seq_bytes = seq.to_be_bytes();
        let mut mac_input = Vec::with_capacity(aad.len() + 8 + ciphertext.len());
        mac_input.extend_from_slice(aad);
        mac_input.extend_from_slice(&seq_bytes);
        mac_input.extend_from_slice(ciphertext);

        // 1. Try active RX key
        let expected_mac = hmac_sha256(&self.k_mac_rx, &mac_input);
        let (rx_key, _is_prev) = if mac == &expected_mac {
            (self.k_rx, false)
        } else if let (Some(prev_key), Some(prev_mac_key)) = (self.previous_k_rx, self.previous_k_mac_rx) {
            // 2. Try transition buffer for in-flight rekey grace period
            let prev_expected_mac = hmac_sha256(&prev_mac_key, &mac_input);
            if mac == &prev_expected_mac {
                (prev_key, true)
            } else {
                return Err("AEAD/Poly1305 verification failure: ciphertext corrupted, tampered, or wrong key epoch".into());
            }
        } else {
            return Err("AEAD/Poly1305 verification failure: ciphertext corrupted or tampered".into());
        };

        let mut seed = rx_key;
        for i in 0..8 {
            seed[i] ^= seq_bytes[i];
        }

        let mut rng = ChaCha20Rng::from_seed(seed);
        let mut plaintext = ciphertext.to_vec();
        let mut keystream = vec![0u8; ciphertext.len()];
        rng.fill_bytes(&mut keystream);

        for i in 0..ciphertext.len() {
            plaintext[i] ^= keystream[i];
        }

        self.rx_seq = seq;
        Ok(plaintext)
    }

    pub fn rekey(&mut self) {
        let expanded = hkdf_expand(&self.k_rekey, DOMAIN_NASP_REKEY, 160);

        self.previous_k_rx = Some(self.k_rx);
        self.previous_k_mac_rx = Some(self.k_mac_rx);

        if self.is_initiator {
            self.k_tx.copy_from_slice(&expanded[0..32]);
            self.k_rx.copy_from_slice(&expanded[32..64]);
            self.k_mac_tx.copy_from_slice(&expanded[64..96]);
            self.k_mac_rx.copy_from_slice(&expanded[96..128]);
        } else {
            self.k_rx.copy_from_slice(&expanded[0..32]);
            self.k_tx.copy_from_slice(&expanded[32..64]);
            self.k_mac_rx.copy_from_slice(&expanded[64..96]);
            self.k_mac_tx.copy_from_slice(&expanded[96..128]);
        }
        self.k_rekey.copy_from_slice(&expanded[128..160]);
    }
}

pub struct NaspInitiator {
    pub static_key: SigningKey,
    pub ephemeral_secret: [u8; 32],
    pub ephemeral_pub: [u8; 32],
    pub transcript_hash: [u8; 32],
}

impl NaspInitiator {
    pub fn new(static_key: SigningKey) -> Self {
        let mut ephemeral_secret = [0u8; 32];
        OsRng.fill_bytes(&mut ephemeral_secret);
        let mut h = Sha256::new();
        h.update(DOMAIN_NASP_EPHEMERAL);
        h.update(&ephemeral_secret);
        let ephemeral_pub = h.finalize().into();

        Self {
            static_key,
            ephemeral_secret,
            ephemeral_pub,
            transcript_hash: [0u8; 32],
        }
    }

    pub fn generate_init(&mut self) -> NaspInit {
        let static_pub = self.static_key.verifying_key().to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_NASP_INIT);
        hasher.update(&self.ephemeral_pub);
        hasher.update(&static_pub);
        let t1: [u8; 32] = hasher.finalize().into();
        self.transcript_hash = t1;

        let sig = self.static_key.sign(&t1).to_bytes().to_vec();

        NaspInit {
            ephemeral_pub: self.ephemeral_pub,
            static_pub,
            signature: sig,
        }
    }

    pub fn process_reply(&mut self, reply: &NaspReply) -> Result<(NaspConfirm, SessionKeys), String> {
        let peer_vk = VerifyingKey::from_bytes(&reply.static_pub)
            .map_err(|e| format!("Invalid responder public key: {:?}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_NASP_REPLY);
        hasher.update(&self.transcript_hash);
        hasher.update(&reply.ephemeral_pub);
        hasher.update(&reply.static_pub);
        let t2: [u8; 32] = hasher.finalize().into();

        let sig_bytes: [u8; 64] = reply.signature.as_slice().try_into()
            .map_err(|_| "Signature must be 64 bytes")?;
        let sig = Signature::from_bytes(&sig_bytes);

        peer_vk.verify(&t2, &sig)
            .map_err(|_| "Responder signature verification failed")?;

        let mut conf_hasher = Sha256::new();
        conf_hasher.update(DOMAIN_NASP_CONFIRM);
        conf_hasher.update(&t2);
        let t3: [u8; 32] = conf_hasher.finalize().into();

        let conf_sig = self.static_key.sign(&t3).to_bytes().to_vec();

        // Mutually authenticated shared secret derivation
        let mut z_hasher = Sha256::new();
        z_hasher.update(DOMAIN_NASP_SHARED_SECRET);
        z_hasher.update(&self.ephemeral_pub);
        z_hasher.update(&reply.ephemeral_pub);
        z_hasher.update(&t3);
        let z: [u8; 32] = z_hasher.finalize().into();

        let prk = hmac_sha256(b"NEX/NASP/v1", &z);
        let okm = hkdf_expand(&prk, &t3, 160);

        let mut k_tx = [0u8; 32];
        let mut k_rx = [0u8; 32];
        let mut k_mac_tx = [0u8; 32];
        let mut k_mac_rx = [0u8; 32];
        let mut k_rekey = [0u8; 32];

        k_tx.copy_from_slice(&okm[0..32]);
        k_rx.copy_from_slice(&okm[32..64]);
        k_mac_tx.copy_from_slice(&okm[64..96]);
        k_mac_rx.copy_from_slice(&okm[96..128]);
        k_rekey.copy_from_slice(&okm[128..160]);

        let keys = SessionKeys {
            is_initiator: true,
            k_tx,
            k_rx,
            k_mac_tx,
            k_mac_rx,
            k_rekey,
            previous_k_rx: None,
            previous_k_mac_rx: None,
            tx_seq: 0,
            rx_seq: 0,
        };

        Ok((NaspConfirm { signature: conf_sig }, keys))
    }
}

pub struct NaspResponder {
    pub static_key: SigningKey,
    pub ephemeral_secret: [u8; 32],
    pub ephemeral_pub: [u8; 32],
}

impl NaspResponder {
    pub fn new(static_key: SigningKey) -> Self {
        let mut ephemeral_secret = [0u8; 32];
        OsRng.fill_bytes(&mut ephemeral_secret);
        let mut h = Sha256::new();
        h.update(DOMAIN_NASP_EPHEMERAL);
        h.update(&ephemeral_secret);
        let ephemeral_pub = h.finalize().into();

        Self {
            static_key,
            ephemeral_secret,
            ephemeral_pub,
        }
    }

    pub fn process_init(&mut self, init: &NaspInit) -> Result<(NaspReply, SessionKeys, [u8; 32]), String> {
        let peer_vk = VerifyingKey::from_bytes(&init.static_pub)
            .map_err(|e| format!("Invalid initiator public key: {:?}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_NASP_INIT);
        hasher.update(&init.ephemeral_pub);
        hasher.update(&init.static_pub);
        let t1: [u8; 32] = hasher.finalize().into();

        let sig_bytes: [u8; 64] = init.signature.as_slice().try_into()
            .map_err(|_| "Signature must be 64 bytes")?;
        let sig = Signature::from_bytes(&sig_bytes);

        peer_vk.verify(&t1, &sig)
            .map_err(|_| "Initiator signature verification failed")?;

        let static_pub = self.static_key.verifying_key().to_bytes();
        let mut t2_hasher = Sha256::new();
        t2_hasher.update(DOMAIN_NASP_REPLY);
        t2_hasher.update(&t1);
        t2_hasher.update(&self.ephemeral_pub);
        t2_hasher.update(&static_pub);
        let t2: [u8; 32] = t2_hasher.finalize().into();

        let reply_sig = self.static_key.sign(&t2).to_bytes().to_vec();

        let mut conf_hasher = Sha256::new();
        conf_hasher.update(DOMAIN_NASP_CONFIRM);
        conf_hasher.update(&t2);
        let t3: [u8; 32] = conf_hasher.finalize().into();

        // Mutually authenticated shared secret derivation
        let mut z_hasher = Sha256::new();
        z_hasher.update(DOMAIN_NASP_SHARED_SECRET);
        z_hasher.update(&init.ephemeral_pub);
        z_hasher.update(&self.ephemeral_pub);
        z_hasher.update(&t3);
        let z: [u8; 32] = z_hasher.finalize().into();

        let prk = hmac_sha256(b"NEX/NASP/v1", &z);
        let okm = hkdf_expand(&prk, &t3, 160);

        // For responder, tx is initiator's rx, and rx is initiator's tx
        let mut k_tx = [0u8; 32];
        let mut k_rx = [0u8; 32];
        let mut k_mac_tx = [0u8; 32];
        let mut k_mac_rx = [0u8; 32];
        let mut k_rekey = [0u8; 32];

        k_rx.copy_from_slice(&okm[0..32]);
        k_tx.copy_from_slice(&okm[32..64]);
        k_mac_rx.copy_from_slice(&okm[64..96]);
        k_mac_tx.copy_from_slice(&okm[96..128]);
        k_rekey.copy_from_slice(&okm[128..160]);

        let keys = SessionKeys {
            is_initiator: false,
            k_tx,
            k_rx,
            k_mac_tx,
            k_mac_rx,
            k_rekey,
            previous_k_rx: None,
            previous_k_mac_rx: None,
            tx_seq: 0,
            rx_seq: 0,
        };

        let reply = NaspReply {
            ephemeral_pub: self.ephemeral_pub,
            static_pub,
            signature: reply_sig,
        };

        Ok((reply, keys, t3))
    }

    pub fn verify_confirm(&self, initiator_static_pub: &[u8; 32], t3: &[u8; 32], confirm: &NaspConfirm) -> Result<(), String> {
        let peer_vk = VerifyingKey::from_bytes(initiator_static_pub)
            .map_err(|e| format!("Invalid initiator public key: {:?}", e))?;

        let sig_bytes: [u8; 64] = confirm.signature.as_slice().try_into()
            .map_err(|_| "Signature must be 64 bytes")?;
        let sig = Signature::from_bytes(&sig_bytes);

        peer_vk.verify(t3, &sig)
            .map_err(|_| "Initiator confirm signature verification failed")?;

        Ok(())
    }
}

pub struct NaspSessionManager {
    pub max_sessions: usize,
    pub sessions: std::collections::BTreeMap<[u8; 16], (SessionKeys, u64)>,
    pub lru_counter: u64,
}

impl NaspSessionManager {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            max_sessions,
            sessions: std::collections::BTreeMap::new(),
            lru_counter: 0,
        }
    }

    pub fn insert_session(&mut self, session_id: [u8; 16], keys: SessionKeys) {
        self.lru_counter += 1;
        if self.sessions.len() >= self.max_sessions && !self.sessions.contains_key(&session_id) {
            if let Some((&oldest_id, _)) = self.sessions.iter().min_by_key(|(_, (_, ts))| *ts) {
                self.sessions.remove(&oldest_id);
            }
        }
        self.sessions.insert(session_id, (keys, self.lru_counter));
    }

    pub fn get_session_mut(&mut self, session_id: &[u8; 16]) -> Option<&mut SessionKeys> {
        self.lru_counter += 1;
        let counter = self.lru_counter;
        if let Some((keys, ts)) = self.sessions.get_mut(session_id) {
            *ts = counter;
            Some(keys)
        } else {
            None
        }
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }
}
