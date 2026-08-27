use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NamespaceID, ObjectType};
use crate::api::NexAppApi;
use crate::identity::types::{ActorID, CapabilityProof};
use crate::model::{Mutation, MutationBody, CrdtPayload};
use crate::hash::hash_mutation_body;

pub const DOMAIN_CHAT_MSG:     &[u8] = b"NEX/CHAT/MSG/v1";
pub const DOMAIN_CHAT_CHANNEL: &[u8] = b"NEX/CHAT/CHANNEL/v1";
pub const DOMAIN_CHAT_CIPHER:  &[u8] = b"NEX/CHAT/CIPHER/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelType {
    Direct1to1,
    GroupMultiParty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberRole {
    Admin,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatChannel {
    pub channel_id: ObjectID,
    pub name: String,
    pub channel_type: ChannelType,
    pub members: BTreeMap<ActorID, MemberRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: ObjectID,
    pub channel_id: ObjectID,
    pub author_actor_id: ActorID,
    pub ciphertext: Vec<u8>,
    pub mentions: Vec<ActorID>,
    pub attachments: Vec<ObjectID>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessageEntry {
    pub message_id: ObjectID,
    pub channel_id: ObjectID,
    pub sender: ActorID,
    pub payload: Vec<u8>,
    pub sequence_index: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ChatEngine {
    pub channels: BTreeMap<String, [u8; 32]>,
    pub read_receipts: BTreeMap<(ObjectID, ActorID), u64>,
    pub message_counter: u64,
}

impl ChatEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send_message(
        &mut self,
        channel_id: ObjectID,
        sender: ActorID,
        payload: Vec<u8>,
        epoch: u64,
    ) -> (ChatMessageEntry, Mutation) {
        self.message_counter += 1;
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_CHAT_MSG);
        hasher.update(&channel_id);
        hasher.update(&sender);
        hasher.update(&payload);
        let message_id: ObjectID = hasher.finalize().into();

        let entry = ChatMessageEntry {
            message_id,
            channel_id,
            sender,
            payload: payload.clone(),
            sequence_index: self.message_counter,
        };

        let body = MutationBody {
            author: sender,
            parents: vec![],
            lamport: epoch,
            epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: message_id, value: payload },
        };

        (entry, Mutation { id: hash_mutation_body(&body), body })
    }

    pub fn mark_read(
        &mut self,
        channel_id: ObjectID,
        reader: ActorID,
        sequence_index: u64,
        epoch: u64,
    ) -> Mutation {
        self.read_receipts.insert((channel_id, reader), sequence_index);
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/CHAT/RECEIPT/v1");
        hasher.update(&channel_id);
        hasher.update(&reader);
        let receipt_id: ObjectID = hasher.finalize().into();

        let body = MutationBody {
            author: reader,
            parents: vec![],
            lamport: epoch,
            epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: receipt_id, value: sequence_index.to_le_bytes().to_vec() },
        };
        Mutation { id: hash_mutation_body(&body), body }
    }
}

pub struct NexChatEngine<A: NexAppApi> {
    pub namespace_id: NamespaceID,
    pub local_actor_id: ActorID,
    pub api: A,
    pub channels: BTreeMap<ObjectID, ChatChannel>,
    pub messages: BTreeMap<ObjectID, ChatMessage>,
    pub reactions: BTreeMap<ObjectID, BTreeMap<String, Vec<ActorID>>>,
}

impl<A: NexAppApi> NexChatEngine<A> {
    pub fn new(namespace_id: NamespaceID, local_actor_id: ActorID, api: A) -> Self {
        Self {
            namespace_id,
            local_actor_id,
            api,
            channels: BTreeMap::new(),
            messages: BTreeMap::new(),
            reactions: BTreeMap::new(),
        }
    }

    pub fn encrypt_payload(plaintext: &[u8], channel_key: &[u8; 32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(plaintext.len() + 32);
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_CHAT_CIPHER);
        hasher.update(channel_key);
        let mask: [u8; 32] = hasher.finalize().into();

        for (i, &b) in plaintext.iter().enumerate() {
            out.push(b ^ mask[i % 32]);
        }
        // Append simple HMAC/checksum (32 bytes)
        let mut mac_hasher = Sha256::new();
        mac_hasher.update(b"NEX/CHAT/MAC/v1");
        mac_hasher.update(channel_key);
        mac_hasher.update(&out);
        let mac: [u8; 32] = mac_hasher.finalize().into();
        out.extend_from_slice(&mac);
        out
    }

    pub fn decrypt_payload(ciphertext: &[u8], channel_key: &[u8; 32]) -> Result<Vec<u8>, String> {
        if ciphertext.len() < 32 {
            return Err("Ciphertext too short".into());
        }
        let (cipher_body, mac) = ciphertext.split_at(ciphertext.len() - 32);
        let mut mac_hasher = Sha256::new();
        mac_hasher.update(b"NEX/CHAT/MAC/v1");
        mac_hasher.update(channel_key);
        mac_hasher.update(cipher_body);
        let expected_mac = mac_hasher.finalize();
        if expected_mac.as_slice() != mac {
            return Err("MAC verification failed".into());
        }

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_CHAT_CIPHER);
        hasher.update(channel_key);
        let mask: [u8; 32] = hasher.finalize().into();

        let mut out = Vec::with_capacity(cipher_body.len());
        for (i, &b) in cipher_body.iter().enumerate() {
            out.push(b ^ mask[i % 32]);
        }
        Ok(out)
    }

    pub fn create_channel(
        &mut self,
        name: &str,
        channel_type: ChannelType,
        extra_members: Vec<(ActorID, MemberRole)>,
    ) -> Result<ObjectID, String> {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_CHAT_CHANNEL);
        hasher.update(&self.namespace_id);
        hasher.update(name.as_bytes());
        let channel_id: ObjectID = hasher.finalize().into();

        let mut members = BTreeMap::new();
        members.insert(self.local_actor_id, MemberRole::Admin);
        for (m, r) in extra_members {
            members.insert(m, r);
        }

        let channel = ChatChannel {
            channel_id,
            name: name.to_string(),
            channel_type,
            members,
        };

        let payload = bincode::serialize(&channel).map_err(|e| e.to_string())?;
        let metadata = BTreeMap::from([
            ("name".to_string(), name.to_string()),
        ]);

        let obj_id = self.api.create_object(
            self.namespace_id,
            ObjectType::ChatChannel,
            metadata,
            payload,
        ).map_err(|e| format!("{:?}", e))?;

        self.channels.insert(obj_id, channel);
        Ok(obj_id)
    }

    pub fn send_message(
        &mut self,
        channel_id: ObjectID,
        plaintext: &[u8],
        channel_key: &[u8; 32],
        mentions: Vec<ActorID>,
        attachments: Vec<ObjectID>,
        _proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let ciphertext = Self::encrypt_payload(plaintext, channel_key);

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_CHAT_MSG);
        hasher.update(&channel_id);
        hasher.update(&self.local_actor_id);
        hasher.update(&ciphertext);
        let message_id: ObjectID = hasher.finalize().into();

        let message = ChatMessage {
            message_id,
            channel_id,
            author_actor_id: self.local_actor_id,
            ciphertext,
            mentions,
            attachments,
        };

        let payload = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
        let metadata = BTreeMap::from([
            ("channel_id".to_string(), hex::encode(channel_id)),
            ("author".to_string(), hex::encode(self.local_actor_id)),
        ]);

        let obj_id = self.api.create_object(
            self.namespace_id,
            ObjectType::ChatMessage,
            metadata,
            payload,
        ).map_err(|e| format!("{:?}", e))?;

        self.messages.insert(obj_id, message);
        Ok(obj_id)
    }

    pub fn read_message(
        &self,
        message_id: &ObjectID,
        channel_key: &[u8; 32],
    ) -> Result<(ChatMessage, Vec<u8>), String> {
        let obj = self.api.read_object(message_id).map_err(|e| format!("{:?}", e))?;
        if obj.tombstoned {
            return Err("Message is tombstoned".into());
        }
        let msg: ChatMessage = serde_json::from_slice(&obj.payload_bytes).map_err(|e| e.to_string())?;
        let plaintext = Self::decrypt_payload(&msg.ciphertext, channel_key)?;
        Ok((msg, plaintext))
    }

    pub fn delete_message(
        &mut self,
        message_id: ObjectID,
        proof: Option<CapabilityProof>,
    ) -> Result<(), String> {
        self.api.delete_object(message_id, proof).map_err(|e| format!("{:?}", e))?;
        self.messages.remove(&message_id);
        Ok(())
    }

    pub fn add_reaction(&mut self, message_id: ObjectID, emoji: &str) {
        self.reactions.entry(message_id)
            .or_default()
            .entry(emoji.to_string())
            .or_default()
            .push(self.local_actor_id);
    }

    pub fn get_reactions(&self, message_id: &ObjectID) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        if let Some(emojis) = self.reactions.get(message_id) {
            for (emoji, actors) in emojis {
                counts.insert(emoji.clone(), actors.len());
            }
        }
        counts
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatOutbox {
    pub pending: Vec<(ObjectID, [u8; 32], String)>,
}

impl ChatOutbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spool(&mut self, channel_id: ObjectID, channel_key: [u8; 32], plaintext: &str) {
        self.pending.push((channel_id, channel_key, plaintext.to_string()));
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn flush<A: NexAppApi>(&mut self, engine: &mut NexChatEngine<A>) -> Result<Vec<ObjectID>, String> {
        let mut sent_ids = Vec::new();
        let items = std::mem::take(&mut self.pending);
        for (channel_id, channel_key, plaintext) in items {
            let oid = engine.send_message(channel_id, plaintext.as_bytes(), &channel_key, vec![], vec![], None)?;
            sent_ids.push(oid);
        }
        Ok(sent_ids)
    }
}
