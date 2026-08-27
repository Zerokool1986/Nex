use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::runtime::node::NexNode;
use crate::sync::anti_entropy::{AntiEntropyEngine, SyncStreamBatch};
use crate::object::types::NexObject;
use crate::identity::types::ActorID;

pub const NEX_SOCKET_MAGIC: &[u8; 4] = b"NXSK";

#[derive(Debug, Clone)]
pub struct SocketPeerSession {
    pub remote_addr: SocketAddr,
    pub remote_actor_id: ActorID,
    pub authenticated: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SocketSyncPayload {
    pub batches: Vec<SyncStreamBatch>,
    pub objects: Vec<NexObject>,
}

pub struct LanTcpTransportServer {
    pub bind_addr: SocketAddr,
    listener: Option<TcpListener>,
    is_running: Arc<Mutex<bool>>,
}

impl LanTcpTransportServer {
    pub fn bind(addr_str: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(addr_str)
            .map_err(|e| format!("Failed to bind TCP listener to {}: {}", addr_str, e))?;
        let local_addr = listener.local_addr()
            .map_err(|e| format!("Failed to get local address: {}", e))?;
        listener.set_nonblocking(true)
            .map_err(|e| format!("Failed to set nonblocking: {}", e))?;

        Ok(Self {
            bind_addr: local_addr,
            listener: Some(listener),
            is_running: Arc::new(Mutex::new(true)),
        })
    }

    /// Accept incoming TCP synchronization connections and process one SMT sync round
    pub fn poll_and_sync_one(&self, node: &mut NexNode) -> Result<Option<SocketAddr>, String> {
        if let Some(ref listener) = self.listener {
            match listener.accept() {
                Ok((mut stream, peer_addr)) => {
                    stream.set_nonblocking(false)
                        .map_err(|e| format!("Failed to set blocking: {}", e))?;
                    stream.set_read_timeout(Some(Duration::from_millis(5000))).ok();
                    stream.set_write_timeout(Some(Duration::from_millis(5000))).ok();

                    // Read 4-byte magic
                    let mut magic = [0u8; 4];
                    stream.read_exact(&mut magic)
                        .map_err(|e| format!("Peer handshake magic read failed: {}", e))?;
                    if &magic != NEX_SOCKET_MAGIC {
                        return Err("Invalid socket protocol magic".to_string());
                    }

                    // 1. Read peer frontier size & data
                    let mut len_buf = [0u8; 4];
                    stream.read_exact(&mut len_buf)
                        .map_err(|e| format!("Failed to read frontier len: {}", e))?;
                    let len = u32::from_le_bytes(len_buf) as usize;

                    let mut peer_frontier_bytes = vec![0u8; len];
                    stream.read_exact(&mut peer_frontier_bytes)
                        .map_err(|e| format!("Failed to read peer frontier: {}", e))?;

                    let peer_frontiers: Vec<[u8; 32]> = bincode::deserialize(&peer_frontier_bytes)
                        .map_err(|e| format!("Failed to deserialize peer frontiers: {}", e))?;

                    // 2. Generate missing batches and objects for peer
                    let session_id = [0x99; 16];
                    let batches = AntiEntropyEngine::generate_batches_for_peer(
                        node,
                        session_id,
                        &peer_frontiers,
                        100,
                    );

                    let objects: Vec<NexObject> = node.state.object_store.values().cloned().collect();
                    let sync_payload = SocketSyncPayload { batches, objects };

                    let payload = bincode::serialize(&sync_payload)
                        .map_err(|e| format!("Failed to serialize sync payload: {}", e))?;
                    let resp_len = (payload.len() as u32).to_le_bytes();

                    stream.write_all(&resp_len)
                        .map_err(|e| format!("Failed to write resp len: {}", e))?;
                    stream.write_all(&payload)
                        .map_err(|e| format!("Failed to write batch payload: {}", e))?;
                    stream.flush().ok();

                    Ok(Some(peer_addr))
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
                Err(e) => Err(format!("TCP accept error: {}", e)),
            }
        } else {
            Ok(None)
        }
    }
}

pub struct LanTcpTransportClient;

impl LanTcpTransportClient {
    /// Connects to a physical remote NEX node over real TCP socket and executes SMT anti-entropy sync
    pub fn sync_with_remote_node(
        local_node: &mut NexNode,
        remote_addr: SocketAddr,
    ) -> Result<usize, String> {
        let mut stream = TcpStream::connect_timeout(&remote_addr, Duration::from_millis(5000))
            .map_err(|e| format!("Failed to connect to remote peer at {}: {}", remote_addr, e))?;

        stream.set_read_timeout(Some(Duration::from_millis(5000))).ok();
        stream.set_write_timeout(Some(Duration::from_millis(5000))).ok();

        // 1. Send handshake magic
        stream.write_all(NEX_SOCKET_MAGIC)
            .map_err(|e| format!("Failed to send magic: {}", e))?;

        // 2. Send local SMT frontier
        let session_id = [0x99; 16];
        let local_adv = AntiEntropyEngine::generate_advertise(local_node, session_id);
        let frontier_bytes = bincode::serialize(&local_adv.frontier_mutation_ids)
            .map_err(|e| format!("Failed to serialize local frontier: {}", e))?;

        let len_buf = (frontier_bytes.len() as u32).to_le_bytes();
        stream.write_all(&len_buf)
            .map_err(|e| format!("Failed to write frontier len: {}", e))?;
        stream.write_all(&frontier_bytes)
            .map_err(|e| format!("Failed to write frontier bytes: {}", e))?;
        stream.flush().ok();

        // 3. Read response payload
        let mut resp_len_buf = [0u8; 4];
        stream.read_exact(&mut resp_len_buf)
            .map_err(|e| format!("Failed to read batch resp len: {}", e))?;
        let resp_len = u32::from_le_bytes(resp_len_buf) as usize;

        let mut resp_bytes = vec![0u8; resp_len];
        stream.read_exact(&mut resp_bytes)
            .map_err(|e| format!("Failed to read batch bytes: {}", e))?;

        let sync_payload: SocketSyncPayload = bincode::deserialize(&resp_bytes)
            .map_err(|e| format!("Failed to deserialize incoming sync payload: {}", e))?;

        let mut ingested_count = 0;
        for batch in sync_payload.batches {
            if AntiEntropyEngine::ingest_batch(local_node, batch).is_ok() {
                ingested_count += 1;
            }
        }

        // Replicate objects into local ObjectStore with causal versioning check
        for obj in sync_payload.objects {
            let obj_id = obj.object_id;
            let should_insert = match local_node.state.object_store.get(&obj_id) {
                Some(existing) => {
                    if matches!(existing.object_type, crate::object::types::ObjectType::Synthetic(_)) && !matches!(obj.object_type, crate::object::types::ObjectType::Synthetic(_)) {
                        true
                    } else if !matches!(existing.object_type, crate::object::types::ObjectType::Synthetic(_)) && matches!(obj.object_type, crate::object::types::ObjectType::Synthetic(_)) {
                        obj.created_epoch > existing.created_epoch ||
                        (obj.created_epoch == existing.created_epoch && obj.created_lamport > existing.created_lamport)
                    } else {
                        obj.created_epoch > existing.created_epoch ||
                        (obj.created_epoch == existing.created_epoch && obj.created_lamport >= existing.created_lamport)
                    }
                }
                None => true,
            };

            if should_insert {
                local_node.state.object_store.insert(obj_id, obj);
            }
            ingested_count += 1;
        }

        Ok(ingested_count)
    }
}
