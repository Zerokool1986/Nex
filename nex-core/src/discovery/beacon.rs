use std::net::{UdpSocket, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::identity::types::ActorID;

pub const DISCOVERY_BEACON_MAGIC: &[u8; 4] = b"NXBC";
pub const DEFAULT_BEACON_PORT: u16 = 42424;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryBeacon {
    pub magic: [u8; 4],
    pub actor_id: ActorID,
    pub tcp_port: u16,
    pub blinded_topic: [u8; 32],
    pub timestamp_epoch: u64,
    pub node_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeer {
    pub actor_id: ActorID,
    pub addr: SocketAddr,
    pub tcp_sync_addr: SocketAddr,
    pub blinded_topic: [u8; 32],
    pub last_seen_epoch: u64,
    pub node_name: String,
}

impl DiscoveryBeacon {
    pub fn new(actor_id: ActorID, tcp_port: u16, blinded_topic: [u8; 32], node_name: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            magic: *DISCOVERY_BEACON_MAGIC,
            actor_id,
            tcp_port,
            blinded_topic,
            timestamp_epoch: timestamp,
            node_name: node_name.to_string(),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("Beacon serialization failed: {}", e))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, String> {
        let beacon: Self = bincode::deserialize(bytes)
            .map_err(|e| format!("Beacon deserialization failed: {}", e))?;
        if &beacon.magic != DISCOVERY_BEACON_MAGIC {
            return Err("Invalid beacon magic bytes".to_string());
        }
        Ok(beacon)
    }
}

pub struct LanBeaconService {
    socket: UdpSocket,
    pub local_beacon: DiscoveryBeacon,
    broadcast_addr: SocketAddr,
}

impl LanBeaconService {
    pub fn bind(local_beacon: DiscoveryBeacon, bind_port: u16, target_broadcast_port: u16) -> Result<Self, String> {
        let bind_addr = format!("0.0.0.0:{}", bind_port);
        let socket = UdpSocket::bind(&bind_addr)
            .map_err(|e| format!("Failed to bind UDP socket to {}: {}", bind_addr, e))?;

        socket.set_broadcast(true).map_err(|e| format!("Failed to enable broadcast: {}", e))?;
        socket.set_nonblocking(true).map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        let broadcast_addr: SocketAddr = format!("255.255.255.255:{}", target_broadcast_port)
            .parse()
            .map_err(|e| format!("Invalid broadcast address: {}", e))?;

        Ok(Self {
            socket,
            local_beacon,
            broadcast_addr,
        })
    }

    pub fn broadcast_announcement(&self) -> Result<usize, String> {
        let payload = self.local_beacon.serialize()?;
        self.socket.send_to(&payload, self.broadcast_addr)
            .map_err(|e| format!("Failed to broadcast beacon: {}", e))
    }

    pub fn poll_discovered_peers(&self) -> Vec<DiscoveredPeer> {
        let mut peers = Vec::new();
        let mut buf = [0u8; 1024];

        while let Ok((bytes_read, peer_addr)) = self.socket.recv_from(&mut buf) {
            if let Ok(beacon) = DiscoveryBeacon::deserialize(&buf[..bytes_read]) {
                // Ignore self-announcements
                if beacon.actor_id != self.local_beacon.actor_id {
                    let mut tcp_sync_addr = peer_addr;
                    tcp_sync_addr.set_port(beacon.tcp_port);

                    peers.push(DiscoveredPeer {
                        actor_id: beacon.actor_id,
                        addr: peer_addr,
                        tcp_sync_addr,
                        blinded_topic: beacon.blinded_topic,
                        last_seen_epoch: beacon.timestamp_epoch,
                        node_name: beacon.node_name,
                    });
                }
            }
        }

        peers
    }
}
