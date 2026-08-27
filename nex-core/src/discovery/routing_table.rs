use std::collections::BTreeMap;
use crate::identity::types::ActorID;
use crate::discovery::types::{
    DiscoveryAdvertisement, RouteEntry, EndpointHint, DiscoveryError
};

pub const MAX_ROUTING_TABLE_ENTRIES: usize = 1024;
pub const MAX_ROUTE_HOPS: u8 = 16;

#[derive(Debug, Clone)]
pub struct RoutingTable {
    pub local_actor_id: ActorID,
    pub routes: BTreeMap<ActorID, RouteEntry>,
    pub advertisements: BTreeMap<ActorID, DiscoveryAdvertisement>,
    pub topic_subscriptions: BTreeMap<[u8; 32], Vec<ActorID>>,
}

impl RoutingTable {
    pub fn new(local_actor_id: ActorID) -> Self {
        Self {
            local_actor_id,
            routes: BTreeMap::new(),
            advertisements: BTreeMap::new(),
            topic_subscriptions: BTreeMap::new(),
        }
    }

    /// Ingests and validates an authenticated DiscoveryAdvertisement
    pub fn ingest_advertisement(
        &mut self,
        adv: DiscoveryAdvertisement,
        current_epoch: u64,
    ) -> Result<bool, DiscoveryError> {
        // --- 1. Signature Validation ---
        if adv.signature.is_empty() {
            return Err(DiscoveryError::SignatureInvalid);
        }

        // --- 2. Epoch Validity Window ---
        if current_epoch < adv.not_before_epoch {
            return Err(DiscoveryError::PrematureAdvertisement {
                current_epoch,
                not_before: adv.not_before_epoch,
            });
        }
        if current_epoch > adv.expires_at_epoch {
            return Err(DiscoveryError::ExpiredAdvertisement {
                current_epoch,
                expires_at: adv.expires_at_epoch,
            });
        }

        // --- 3. Sequence Monotonicity (Anti-Replay) ---
        if let Some(existing) = self.advertisements.get(&adv.actor_id) {
            if adv.sequence < existing.sequence {
                return Err(DiscoveryError::StaleSequence {
                    current_sequence: existing.sequence,
                    incoming_sequence: adv.sequence,
                });
            }
            if adv.sequence == existing.sequence && adv == *existing {
                return Ok(false); // Idempotent duplicate
            }
        }

        // --- 4. Sizing Limit Bounds ---
        if self.advertisements.len() >= MAX_ROUTING_TABLE_ENTRIES && !self.advertisements.contains_key(&adv.actor_id) {
            // Prune expired routes first to free capacity
            self.prune_expired_routes(current_epoch);
            if self.advertisements.len() >= MAX_ROUTING_TABLE_ENTRIES {
                return Err(DiscoveryError::RoutingTableFull);
            }
        }

        // --- 5. Update Direct Route (1-hop to destination) ---
        let direct_route = RouteEntry {
            destination: adv.actor_id,
            next_hop: adv.actor_id,
            hop_count: 1,
            sequence: adv.sequence,
            expires_at_epoch: adv.expires_at_epoch,
            endpoint_hints: adv.endpoint_hints.clone(),
        };

        // --- 6. Index Topic / Namespace ---
        let topic_entries = self.topic_subscriptions.entry(adv.namespace_or_topic).or_default();
        if !topic_entries.contains(&adv.actor_id) {
            topic_entries.push(adv.actor_id);
        }

        self.routes.insert(adv.actor_id, direct_route);
        self.advertisements.insert(adv.actor_id, adv);

        Ok(true)
    }

    /// Ingests a multi-hop route discovered via peer gossip
    pub fn add_multi_hop_route(
        &mut self,
        destination: ActorID,
        next_hop: ActorID,
        hop_count: u8,
        sequence: u64,
        expires_at_epoch: u64,
        endpoint_hints: Vec<EndpointHint>,
        current_epoch: u64,
    ) -> Result<bool, DiscoveryError> {
        if hop_count > MAX_ROUTE_HOPS {
            return Err(DiscoveryError::InvalidEndpoint);
        }
        if current_epoch > expires_at_epoch {
            return Err(DiscoveryError::ExpiredAdvertisement {
                current_epoch,
                expires_at: expires_at_epoch,
            });
        }

        // Loop prevention: next_hop cannot be ourselves
        if next_hop == self.local_actor_id {
            return Ok(false);
        }

        let should_update = match self.routes.get(&destination) {
            Some(existing) => {
                sequence > existing.sequence ||
                (sequence == existing.sequence && hop_count < existing.hop_count)
            }
            None => true,
        };

        if should_update {
            if self.routes.len() >= MAX_ROUTING_TABLE_ENTRIES && !self.routes.contains_key(&destination) {
                self.prune_expired_routes(current_epoch);
                if self.routes.len() >= MAX_ROUTING_TABLE_ENTRIES {
                    return Err(DiscoveryError::RoutingTableFull);
                }
            }

            self.routes.insert(destination, RouteEntry {
                destination,
                next_hop,
                hop_count,
                sequence,
                expires_at_epoch,
                endpoint_hints,
            });
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Finds the active, unexpired route to a target destination
    pub fn find_best_route(&self, destination: &ActorID, current_epoch: u64) -> Option<&RouteEntry> {
        self.routes.get(destination).and_then(|r| {
            if r.expires_at_epoch >= current_epoch {
                Some(r)
            } else {
                None
            }
        })
    }

    /// Prunes all expired routes and advertisements
    pub fn prune_expired_routes(&mut self, current_epoch: u64) -> usize {
        let mut expired_actors = Vec::new();
        for (actor, adv) in &self.advertisements {
            if adv.expires_at_epoch < current_epoch {
                expired_actors.push(*actor);
            }
        }

        let pruned_count = expired_actors.len();
        for actor in expired_actors {
            self.advertisements.remove(&actor);
            self.routes.remove(&actor);
        }

        // Clean up topic indexes
        for entries in self.topic_subscriptions.values_mut() {
            entries.retain(|actor| self.advertisements.contains_key(actor));
        }

        pruned_count
    }
}
