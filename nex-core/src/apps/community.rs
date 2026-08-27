use std::collections::{BTreeMap, BTreeSet};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::object::types::{ObjectID, NamespaceID, ObjectType};
use crate::api::NexAppApi;
use crate::identity::types::{ActorID, CapabilityProof, OP_OBJECT_TOMBSTONE};
use crate::model::{Mutation, MutationBody, CrdtPayload};
use crate::hash::hash_mutation_body;

pub const DOMAIN_COMMUNITY_POST:  &[u8] = b"NEX/COMMUNITY/POST/v1";
pub const DOMAIN_COMMUNITY_REPLY: &[u8] = b"NEX/COMMUNITY/REPLY/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CommunityRole {
    Guest = 0,
    Member = 1,
    Moderator = 2,
    Admin = 3,
    Owner = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Community {
    pub community_id: ObjectID,
    pub name: String,
    pub description: String,
    pub owner_actor_id: ActorID,
    pub created_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityChannel {
    pub channel_id: ObjectID,
    pub community_id: ObjectID,
    pub name: String,
    pub is_private: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityPost {
    pub post_id: ObjectID,
    pub channel_id: ObjectID,
    pub author_actor_id: ActorID,
    pub title: String,
    pub content: String,
    pub created_epoch: u64,
    pub is_pinned: bool,
    pub is_locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityReply {
    pub reply_id: ObjectID,
    pub post_id: ObjectID,
    pub parent_reply_id: Option<ObjectID>,
    pub author_actor_id: ActorID,
    pub body_content: String,
    pub created_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct CommunityEngine {
    pub namespace_id: NamespaceID,
    pub local_actor_id: ActorID,
    pub members: BTreeMap<ActorID, (CommunityRole, u64)>,
    pub tombstoned_messages: BTreeMap<ObjectID, u64>,
}

impl CommunityEngine {
    pub fn new(namespace_id: NamespaceID, local_actor_id: ActorID) -> Self {
        Self {
            namespace_id,
            local_actor_id,
            members: BTreeMap::new(),
            tombstoned_messages: BTreeMap::new(),
        }
    }

    pub fn add_member(
        &mut self,
        member: ActorID,
        role: CommunityRole,
        epoch: u64,
    ) -> Mutation {
        self.members.insert(member, (role, epoch));
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/COMMUNITY/MEMBER/v1");
        hasher.update(&self.namespace_id);
        hasher.update(&member);
        let member_obj_id: ObjectID = hasher.finalize().into();

        let body = MutationBody {
            author: self.local_actor_id,
            parents: vec![],
            lamport: epoch,
            epoch,
            is_resurrect: false,
            payload: CrdtPayload::AddLWW { id: member_obj_id, value: vec![role as u8] },
        };
        Mutation { id: hash_mutation_body(&body), body }
    }

    pub fn tombstone_message(
        &mut self,
        message_id: ObjectID,
        proof: &CapabilityProof,
        epoch: u64,
    ) -> Result<Mutation, String> {
        let empty_rev = BTreeMap::new();
        let auth_ok = crate::identity::verifier::verify_capability_chain(
            proof,
            OP_OBJECT_TOMBSTONE,
            &self.namespace_id,
            Some(&message_id),
            epoch,
            &empty_rev,
            &self.local_actor_id,
        );
        if auth_ok.is_err() {
            return Err("Unauthorized".into());
        }

        self.tombstoned_messages.insert(message_id, epoch);
        let body = MutationBody {
            author: proof.token.subject,
            parents: vec![],
            lamport: epoch,
            epoch,
            is_resurrect: false,
            payload: CrdtPayload::Tombstone { id: message_id },
        };
        Ok(Mutation { id: hash_mutation_body(&body), body })
    }
}

pub struct NexCommunityEngine<A: NexAppApi> {
    pub local_actor_id: ActorID,
    pub api: A,
    pub communities: BTreeMap<ObjectID, Community>,
    pub channels: BTreeMap<ObjectID, CommunityChannel>,
    pub posts: BTreeMap<ObjectID, CommunityPost>,
    pub replies: BTreeMap<ObjectID, CommunityReply>,
    pub roles: BTreeMap<ObjectID, BTreeMap<ActorID, CommunityRole>>,
    pub reactions: BTreeMap<ObjectID, BTreeMap<String, BTreeSet<ActorID>>>,
}

impl<A: NexAppApi> NexCommunityEngine<A> {
    pub fn new(local_actor_id: ActorID, api: A) -> Self {
        Self {
            local_actor_id,
            api,
            communities: BTreeMap::new(),
            channels: BTreeMap::new(),
            posts: BTreeMap::new(),
            replies: BTreeMap::new(),
            roles: BTreeMap::new(),
            reactions: BTreeMap::new(),
        }
    }

    pub fn assign_role(
        &mut self,
        community_id: ObjectID,
        target_actor: ActorID,
        role: CommunityRole,
    ) -> Result<(), String> {
        let caller_role = self.get_role(&community_id, &self.local_actor_id);
        if caller_role < CommunityRole::Admin {
            return Err("Unauthorized: Must be at least Admin to assign roles".into());
        }
        if caller_role <= role && caller_role != CommunityRole::Owner {
            return Err("Unauthorized: Cannot assign a role equal or higher than your own".into());
        }

        self.roles.entry(community_id)
            .or_default()
            .insert(target_actor, role);
        Ok(())
    }

    pub fn get_role(&self, community_id: &ObjectID, actor: &ActorID) -> CommunityRole {
        if let Some(comm) = self.communities.get(community_id) {
            if comm.owner_actor_id == *actor {
                return CommunityRole::Owner;
            }
        }
        self.roles.get(community_id)
            .and_then(|r| r.get(actor).cloned())
            .unwrap_or(CommunityRole::Guest)
    }

    pub fn create_community(
        &mut self,
        namespace_id: NamespaceID,
        name: &str,
        description: &str,
        epoch: u64,
        _proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/COMMUNITY/META/v1");
        hasher.update(name.as_bytes());
        hasher.update(&self.local_actor_id);
        hasher.update(&epoch.to_le_bytes());
        let community_id: ObjectID = hasher.finalize().into();

        let comm = Community {
            community_id,
            name: name.to_string(),
            description: description.to_string(),
            owner_actor_id: self.local_actor_id,
            created_epoch: epoch,
        };

        let payload = serde_json::to_vec(&comm).map_err(|e| e.to_string())?;
        let metadata = BTreeMap::from([
            ("name".to_string(), name.to_string()),
            ("owner".to_string(), hex::encode(self.local_actor_id)),
        ]);

        let obj_id = self.api.create_object(
            namespace_id,
            ObjectType::Community,
            metadata,
            payload,
        ).map_err(|e| format!("{:?}", e))?;

        self.communities.insert(obj_id, comm);
        self.roles.entry(obj_id).or_default().insert(self.local_actor_id, CommunityRole::Owner);
        Ok(obj_id)
    }

    pub fn create_channel(
        &mut self,
        namespace_id: NamespaceID,
        community_id: ObjectID,
        name: &str,
        is_private: bool,
        _proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let role = self.get_role(&community_id, &self.local_actor_id);
        if role < CommunityRole::Admin {
            return Err("Unauthorized: Must be Admin to create channel".into());
        }

        let mut hasher = Sha256::new();
        hasher.update(b"NEX/COMMUNITY/CHANNEL/v1");
        hasher.update(&community_id);
        hasher.update(name.as_bytes());
        let channel_id: ObjectID = hasher.finalize().into();

        let channel = CommunityChannel {
            channel_id,
            community_id,
            name: name.to_string(),
            is_private,
        };

        let payload = serde_json::to_vec(&channel).map_err(|e| e.to_string())?;
        let metadata = BTreeMap::from([
            ("community_id".to_string(), hex::encode(community_id)),
            ("name".to_string(), name.to_string()),
        ]);

        let obj_id = self.api.create_object(
            namespace_id,
            ObjectType::Community,
            metadata,
            payload,
        ).map_err(|e| format!("{:?}", e))?;

        self.channels.insert(obj_id, channel);
        Ok(obj_id)
    }

    pub fn create_post(
        &mut self,
        namespace_id: NamespaceID,
        channel_id: ObjectID,
        title: &str,
        content: &str,
        epoch: u64,
        _proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_COMMUNITY_POST);
        hasher.update(&channel_id);
        hasher.update(&self.local_actor_id);
        hasher.update(title.as_bytes());
        hasher.update(&epoch.to_le_bytes());
        let post_id: ObjectID = hasher.finalize().into();

        let post = CommunityPost {
            post_id,
            channel_id,
            author_actor_id: self.local_actor_id,
            title: title.to_string(),
            content: content.to_string(),
            created_epoch: epoch,
            is_pinned: false,
            is_locked: false,
        };

        let payload = serde_json::to_vec(&post).map_err(|e| e.to_string())?;
        let metadata = BTreeMap::from([
            ("channel_id".to_string(), hex::encode(channel_id)),
            ("author".to_string(), hex::encode(self.local_actor_id)),
        ]);

        let obj_id = self.api.create_object(
            namespace_id,
            ObjectType::Community,
            metadata,
            payload,
        ).map_err(|e| format!("{:?}", e))?;

        self.posts.insert(obj_id, post);
        Ok(obj_id)
    }

    pub fn create_reply(
        &mut self,
        namespace_id: NamespaceID,
        post_id: ObjectID,
        parent_reply_id: Option<ObjectID>,
        body_content: &str,
        epoch: u64,
        _proof: Option<CapabilityProof>,
    ) -> Result<ObjectID, String> {
        let post = self.posts.get(&post_id)
            .ok_or_else(|| "Post not found".to_string())?;
        if post.is_locked {
            return Err("Thread is locked".into());
        }

        let mut hasher = Sha256::new();
        hasher.update(DOMAIN_COMMUNITY_REPLY);
        hasher.update(&post_id);
        hasher.update(&self.local_actor_id);
        hasher.update(body_content.as_bytes());
        let reply_id: ObjectID = hasher.finalize().into();

        let reply = CommunityReply {
            reply_id,
            post_id,
            parent_reply_id,
            author_actor_id: self.local_actor_id,
            body_content: body_content.to_string(),
            created_epoch: epoch,
        };

        let payload = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
        let metadata = BTreeMap::from([
            ("post_id".to_string(), hex::encode(post_id)),
            ("author".to_string(), hex::encode(self.local_actor_id)),
        ]);

        let obj_id = self.api.create_object(
            namespace_id,
            ObjectType::Community,
            metadata,
            payload,
        ).map_err(|e| format!("{:?}", e))?;

        self.replies.insert(obj_id, reply);
        Ok(obj_id)
    }

    pub fn lock_post(&mut self, community_id: ObjectID, post_id: ObjectID) -> Result<(), String> {
        let role = self.get_role(&community_id, &self.local_actor_id);
        if role < CommunityRole::Moderator {
            return Err("Unauthorized: Must be at least Moderator to lock posts".into());
        }
        if let Some(post) = self.posts.get_mut(&post_id) {
            post.is_locked = true;
            Ok(())
        } else {
            Err("Post not found".into())
        }
    }

    pub fn pin_post(&mut self, community_id: ObjectID, post_id: ObjectID) -> Result<(), String> {
        let role = self.get_role(&community_id, &self.local_actor_id);
        if role < CommunityRole::Moderator {
            return Err("Unauthorized: Must be at least Moderator to pin posts".into());
        }
        if let Some(post) = self.posts.get_mut(&post_id) {
            post.is_pinned = true;
            Ok(())
        } else {
            Err("Post not found".into())
        }
    }

    pub fn moderate_tombstone_post(
        &mut self,
        community_id: ObjectID,
        post_id: ObjectID,
        proof: Option<CapabilityProof>,
    ) -> Result<(), String> {
        let role = self.get_role(&community_id, &self.local_actor_id);
        if role < CommunityRole::Moderator {
            return Err("Unauthorized: Must be at least Moderator to moderate post".into());
        }
        self.api.delete_object(post_id, proof).map_err(|e| format!("{:?}", e))?;
        self.posts.remove(&post_id);
        Ok(())
    }

    pub fn add_reaction(&mut self, post_id: ObjectID, emoji: &str) {
        self.reactions.entry(post_id)
            .or_default()
            .entry(emoji.to_string())
            .or_default()
            .insert(self.local_actor_id);
    }

    pub fn get_reactions(&self, post_id: &ObjectID) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        if let Some(emojis) = self.reactions.get(post_id) {
            for (emoji, actors) in emojis {
                counts.insert(emoji.clone(), actors.len());
            }
        }
        counts
    }

    pub fn accept_invitation(&mut self, token: CommunityInvitationToken) -> Result<(), String> {
        if token.invited_actor_id != self.local_actor_id {
            return Err("Invitation is not intended for local actor".into());
        }
        let issuer_role = self.get_role(&token.community_id, &token.issuer_actor_id);
        if issuer_role < CommunityRole::Admin {
            return Err("Invitation issuer is not authorized (must be Admin/Owner)".into());
        }
        self.roles.entry(token.community_id).or_default().insert(self.local_actor_id, token.assigned_role);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityInvitationToken {
    pub community_id: ObjectID,
    pub invited_actor_id: ActorID,
    pub assigned_role: CommunityRole,
    pub issuer_actor_id: ActorID,
    pub signature: Vec<u8>,
}

impl CommunityInvitationToken {
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/COMMUNITY/INVITE/v1");
        hasher.update(&self.community_id);
        hasher.update(&self.invited_actor_id);
        hasher.update(&[self.assigned_role as u8]);
        hasher.update(&self.issuer_actor_id);
        hasher.finalize().into()
    }
}
