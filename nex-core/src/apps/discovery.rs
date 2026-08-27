use std::collections::{BTreeMap, BTreeSet};
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerLocator {
    pub actor_id: [u8; 32],
    pub socket_addr: String,
    pub last_seen_epoch: u64,
}

pub struct DhtRoutingTable {
    pub local_actor: [u8; 32],
    pub peers: BTreeMap<[u8; 32], PeerLocator>,
}

impl DhtRoutingTable {
    pub fn new(local_actor: [u8; 32]) -> Self {
        Self {
            local_actor,
            peers: BTreeMap::new(),
        }
    }

    pub fn xor_distance(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        let mut dist = [0u8; 32];
        for i in 0..32 {
            dist[i] = a[i] ^ b[i];
        }
        dist
    }

    pub fn add_peer(&mut self, actor_id: [u8; 32], socket_addr: &str, epoch: u64) {
        self.peers.insert(actor_id, PeerLocator {
            actor_id,
            socket_addr: socket_addr.to_string(),
            last_seen_epoch: epoch,
        });
    }

    pub fn find_closest_nodes(&self, target: &[u8; 32], count: usize) -> Vec<[u8; 32]> {
        let mut sorted: Vec<([u8; 32], [u8; 32])> = self.peers.keys()
            .map(|peer_id| (Self::xor_distance(peer_id, target), *peer_id))
            .collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().take(count).map(|(_, peer_id)| peer_id).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustEdge {
    pub to_actor: [u8; 32],
    pub alias: String,
    pub confidence: f64, // 0.0 to 1.0
}

pub struct WebOfTrustRegistry {
    pub edges: BTreeMap<[u8; 32], Vec<TrustEdge>>,
}

impl WebOfTrustRegistry {
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
        }
    }

    pub fn add_alias(&mut self, from: [u8; 32], to: [u8; 32], alias: &str, confidence: f64) {
        let list = self.edges.entry(from).or_default();
        list.push(TrustEdge {
            to_actor: to,
            alias: alias.to_lowercase(),
            confidence: confidence.clamp(0.0, 1.0),
        });
    }

    pub fn resolve_alias(&self, root: &[u8; 32], alias: &str) -> Option<([u8; 32], f64)> {
        let target_alias = alias.to_lowercase();
        // Direct edge
        if let Some(direct) = self.edges.get(root) {
            for edge in direct {
                if edge.alias == target_alias {
                    return Some((edge.to_actor, edge.confidence));
                }
            }
        }
        // 2-hop transitive search
        if let Some(direct) = self.edges.get(root) {
            for edge in direct {
                if let Some(second_hop) = self.edges.get(&edge.to_actor) {
                    for edge2 in second_hop {
                        if edge2.alias == target_alias {
                            let score = edge.confidence * edge2.confidence * 0.5;
                            return Some((edge2.to_actor, score));
                        }
                    }
                }
            }
        }
        None
    }
}

pub struct InvertedSearchIndex {
    pub index: BTreeMap<String, BTreeSet<[u8; 32]>>,
}

impl InvertedSearchIndex {
    pub fn new() -> Self {
        Self {
            index: BTreeMap::new(),
        }
    }

    pub fn index_document(&mut self, doc_id: [u8; 32], text: &str) {
        for word in text.split_whitespace() {
            let clean: String = word.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if !clean.is_empty() {
                self.index.entry(clean).or_default().insert(doc_id);
            }
        }
    }

    pub fn search(&self, query: &str) -> Vec<[u8; 32]> {
        let words: Vec<String> = query.split_whitespace()
            .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            return Vec::new();
        }

        let mut candidate_counts: BTreeMap<[u8; 32], usize> = BTreeMap::new();
        for word in &words {
            if let Some(doc_set) = self.index.get(word) {
                for doc_id in doc_set {
                    *candidate_counts.entry(*doc_id).or_insert(0) += 1;
                }
            }
        }

        let mut ranked: Vec<([u8; 32], usize)> = candidate_counts.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked.into_iter().map(|(id, _)| id).collect()
    }
}

pub struct TopicPubSub {
    pub topics: BTreeMap<[u8; 32], BTreeSet<[u8; 32]>>,
}

impl TopicPubSub {
    pub fn new() -> Self {
        Self {
            topics: BTreeMap::new(),
        }
    }

    pub fn subscribe(&mut self, topic: [u8; 32], subscriber: [u8; 32]) {
        self.topics.entry(topic).or_default().insert(subscriber);
    }

    pub fn publish(&self, topic: &[u8; 32]) -> Vec<[u8; 32]> {
        self.topics.get(topic).map(|set| set.iter().copied().collect()).unwrap_or_default()
    }
}
