use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use crate::identity::types::ActorID;

/// Local, sovereign blocklist maintained strictly on the user's local node.
/// Decoupled from global identity, community governance, and external network gossip.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalBlocklist {
    pub blocked_actors: BTreeSet<ActorID>,
}

impl PersonalBlocklist {
    pub fn new() -> Self {
        Self {
            blocked_actors: BTreeSet::new(),
        }
    }

    /// Locally blocks an ActorID from direct direct-messaging, pairing, or state ingestion.
    pub fn block_actor(&mut self, actor: ActorID) -> bool {
        self.blocked_actors.insert(actor)
    }

    /// Unblocks a previously blocked ActorID.
    pub fn unblock_actor(&mut self, actor: &ActorID) -> bool {
        self.blocked_actors.remove(actor)
    }

    /// Checks whether an ActorID is blocked locally.
    pub fn is_blocked(&self, actor: &ActorID) -> bool {
        self.blocked_actors.contains(actor)
    }

    /// Returns a list of all locally blocked ActorIDs.
    pub fn list_blocked(&self) -> Vec<ActorID> {
        self.blocked_actors.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_blocklist_lifecycle() {
        let mut blocklist = PersonalBlocklist::new();
        let bad_actor = [0x99; 32];
        let good_actor = [0x11; 32];

        assert!(!blocklist.is_blocked(&bad_actor));
        assert!(blocklist.block_actor(bad_actor));
        assert!(blocklist.is_blocked(&bad_actor));
        assert!(!blocklist.is_blocked(&good_actor));

        // Duplicate block returns false (already present)
        assert!(!blocklist.block_actor(bad_actor));

        // Unblock
        assert!(blocklist.unblock_actor(&bad_actor));
        assert!(!blocklist.is_blocked(&bad_actor));
    }
}
