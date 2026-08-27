use std::collections::{VecDeque, HashMap};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Read, Write};
use sha2::{Sha256, Digest};
use crate::transport::types::{
    TransportGuarantee, TransportPacket, TransportError, FRAME_MAGIC
};
use crate::transport::fragmentation::{fragment_payload, FragmentationReassembler};

pub trait TransportAdapter: Send + Sync {
    fn transport_tag(&self) -> u16;
    fn mtu(&self) -> usize;
    fn guarantee(&self) -> TransportGuarantee;
    fn is_connected(&self) -> bool;
    fn send(&mut self, destination: &[u8], payload: &[u8]) -> Result<(), TransportError>;
    fn poll_incoming(&mut self) -> Option<TransportPacket>;
}

/// Derive 16-byte Reticulum destination hash from 32-byte ActorID
pub fn derive_reticulum_destination_hash(actor_id: &[u8; 32]) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(b"NEX/RNS_DEST/v1");
    hasher.update(actor_id);
    let full: [u8; 32] = hasher.finalize().into();
    let mut out = [0u8; 16];
    out.copy_from_slice(&full[0..16]);
    out
}

/// Mock Reticulum mesh adapter (500-byte MTU, Datagram guarantee)
#[derive(Debug, Default)]
pub struct MockReticulumAdapter {
    pub connected: bool,
    pub outbox: VecDeque<(Vec<u8>, Vec<u8>)>,
    pub inbox: VecDeque<TransportPacket>,
}

impl MockReticulumAdapter {
    pub fn new() -> Self {
        Self {
            connected: true,
            outbox: VecDeque::new(),
            inbox: VecDeque::new(),
        }
    }
}

impl TransportAdapter for MockReticulumAdapter {
    fn transport_tag(&self) -> u16 { 0x01 } // Reticulum
    fn mtu(&self) -> usize { 500 }
    fn guarantee(&self) -> TransportGuarantee { TransportGuarantee::UnreliableDatagram }
    fn is_connected(&self) -> bool { self.connected }

    fn send(&mut self, destination: &[u8], payload: &[u8]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::SendFailed("Reticulum interface offline".into()));
        }
        if payload.len() > self.mtu() {
            return Err(TransportError::MtuExceeded { payload_len: payload.len(), mtu: self.mtu() });
        }
        self.outbox.push_back((destination.to_vec(), payload.to_vec()));
        Ok(())
    }

    fn poll_incoming(&mut self) -> Option<TransportPacket> {
        self.inbox.pop_front()
    }
}

/// Native Reticulum Mesh Transport Adapter with 500B MTU chunking and destination hash routing
pub struct ReticulumNativeAdapter {
    pub local_dest_hash: [u8; 16],
    pub connected: bool,
    pub outbox: VecDeque<(Vec<u8>, Vec<u8>)>,
    pub reassembler: FragmentationReassembler,
    pub inbox: VecDeque<TransportPacket>,
    pub link_mtu: usize,
}

impl ReticulumNativeAdapter {
    pub fn new(local_dest_hash: [u8; 16]) -> Self {
        Self {
            local_dest_hash,
            connected: true,
            outbox: VecDeque::new(),
            reassembler: FragmentationReassembler::new(),
            inbox: VecDeque::new(),
            link_mtu: 500,
        }
    }

    /// Ingests a raw Reticulum chunk (e.g. from physical LoRa interface)
    pub fn ingest_packet(&mut self, source_dest_hash: &[u8], raw_packet: &[u8], current_epoch: u64) -> Result<(), TransportError> {
        if let Some(reassembled_payload) = self.reassembler.ingest_chunk_with_epoch(raw_packet, current_epoch)? {
            if let Ok((tag, _flags, payload)) = crate::transport::types::decode_frame(&reassembled_payload) {
                self.inbox.push_back(TransportPacket {
                    transport_tag: tag,
                    source_address: source_dest_hash.to_vec(),
                    payload,
                });
            }
        }
        Ok(())
    }
}

impl TransportAdapter for ReticulumNativeAdapter {
    fn transport_tag(&self) -> u16 { 0x01 } // Reticulum
    fn mtu(&self) -> usize { 65535 }
    fn guarantee(&self) -> TransportGuarantee { TransportGuarantee::UnreliableDatagram }
    fn is_connected(&self) -> bool { self.connected }

    fn send(&mut self, destination: &[u8], payload: &[u8]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::SendFailed("Reticulum interface offline".into()));
        }

        // 1. Encode into canonical 13-byte wire frame
        let wire_frame = crate::transport::types::encode_frame(self.transport_tag(), 0, payload);

        // 2. Compute 32-byte message ID for fragmentation header
        let mut hasher = Sha256::new();
        hasher.update(b"NEX/RNS_CHUNK/v1");
        hasher.update(&wire_frame);
        let msg_id: [u8; 32] = hasher.finalize().into();

        // 3. Fragment into physical link-MTU packets
        let chunks = fragment_payload(msg_id, &wire_frame, self.link_mtu)?;
        for chunk in chunks {
            self.outbox.push_back((destination.to_vec(), chunk));
        }

        Ok(())
    }

    fn poll_incoming(&mut self) -> Option<TransportPacket> {
        self.inbox.pop_front()
    }
}

/// Mock QUIC transport adapter (65535-byte MTU, Stream guarantee)
#[derive(Debug, Default)]
pub struct MockQuicAdapter {
    pub connected: bool,
    pub outbox: VecDeque<(Vec<u8>, Vec<u8>)>,
    pub inbox: VecDeque<TransportPacket>,
}

impl MockQuicAdapter {
    pub fn new() -> Self {
        Self {
            connected: true,
            outbox: VecDeque::new(),
            inbox: VecDeque::new(),
        }
    }
}

impl TransportAdapter for MockQuicAdapter {
    fn transport_tag(&self) -> u16 { 0x02 } // QUIC
    fn mtu(&self) -> usize { 65535 }
    fn guarantee(&self) -> TransportGuarantee { TransportGuarantee::ReliableStream }
    fn is_connected(&self) -> bool { self.connected }

    fn send(&mut self, destination: &[u8], payload: &[u8]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::SendFailed("QUIC connection closed".into()));
        }
        if payload.len() > self.mtu() {
            return Err(TransportError::MtuExceeded { payload_len: payload.len(), mtu: self.mtu() });
        }
        self.outbox.push_back((destination.to_vec(), payload.to_vec()));
        Ok(())
    }

    fn poll_incoming(&mut self) -> Option<TransportPacket> {
        self.inbox.pop_front()
    }
}

/// Concrete Physical TCP/IP Transport Adapter (4MB MTU, Stream guarantee)
pub struct TcpTransportAdapter {
    pub local_addr: SocketAddr,
    pub listener: Option<TcpListener>,
    pub streams: HashMap<SocketAddr, TcpStream>,
    pub rx_buffers: HashMap<SocketAddr, Vec<u8>>,
    pub inbox: VecDeque<TransportPacket>,
    pub connected: bool,
}

impl TcpTransportAdapter {
    pub fn bind(bind_addr: &str) -> std::io::Result<Self> {
        let listener = TcpListener::bind(bind_addr)?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        Ok(Self {
            local_addr,
            listener: Some(listener),
            streams: HashMap::new(),
            rx_buffers: HashMap::new(),
            inbox: VecDeque::new(),
            connected: true,
        })
    }

    pub fn connect_to(&mut self, target: SocketAddr) -> std::io::Result<()> {
        let stream = TcpStream::connect(target)?;
        stream.set_nonblocking(true)?;
        self.streams.insert(target, stream);
        Ok(())
    }

    pub fn poll_network(&mut self) -> Result<(), TransportError> {
        // 1. Accept incoming TCP connections
        if let Some(ref listener) = self.listener {
            while let Ok((stream, peer_addr)) = listener.accept() {
                let _ = stream.set_nonblocking(true);
                self.streams.insert(peer_addr, stream);
            }
        }

        // 2. Read and decode canonical wire frames from active streams
        let mut dead_peers = Vec::new();
        let mut incoming_packets = Vec::new();

        for (&peer_addr, stream) in self.streams.iter_mut() {
            let rx_buf = self.rx_buffers.entry(peer_addr).or_default();
            let mut chunk = [0u8; 65536];

            loop {
                match stream.read(&mut chunk) {
                    Ok(0) => {
                        dead_peers.push(peer_addr);
                        break;
                    }
                    Ok(n) => {
                        rx_buf.extend_from_slice(&chunk[..n]);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(_) => {
                        dead_peers.push(peer_addr);
                        break;
                    }
                }
            }

            // Stream reassembly loop
            while rx_buf.len() >= 13 {
                if rx_buf[0..2] != FRAME_MAGIC {
                    rx_buf.remove(0);
                    continue;
                }
                let payload_len = u32::from_be_bytes([rx_buf[5], rx_buf[6], rx_buf[7], rx_buf[8]]) as usize;
                if rx_buf.len() < 13 + payload_len {
                    // Incomplete frame, wait for more segments
                    break;
                }
                let full_frame: Vec<u8> = rx_buf.drain(..13 + payload_len).collect();
                if let Ok((tag, _flags, payload)) = crate::transport::types::decode_frame(&full_frame) {
                    incoming_packets.push(TransportPacket {
                        transport_tag: tag,
                        source_address: peer_addr.to_string().into_bytes(),
                        payload,
                    });
                }
            }
        }

        for peer in dead_peers {
            self.streams.remove(&peer);
            self.rx_buffers.remove(&peer);
        }

        for pkt in incoming_packets {
            self.inbox.push_back(pkt);
        }

        Ok(())
    }
}

impl TransportAdapter for TcpTransportAdapter {
    fn transport_tag(&self) -> u16 { 0x03 } // TCP/IP
    fn mtu(&self) -> usize { 4 * 1024 * 1024 } // 4MB stream MTU
    fn guarantee(&self) -> TransportGuarantee { TransportGuarantee::ReliableStream }
    fn is_connected(&self) -> bool { self.connected }

    fn send(&mut self, destination: &[u8], payload: &[u8]) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::SendFailed("TCP adapter offline".into()));
        }
        if payload.len() > self.mtu() {
            return Err(TransportError::MtuExceeded { payload_len: payload.len(), mtu: self.mtu() });
        }

        let dest_str = std::str::from_utf8(destination)
            .map_err(|_| TransportError::SendFailed("Invalid destination address UTF-8".into()))?;
        let dest_addr: SocketAddr = dest_str.parse()
            .map_err(|_| TransportError::SendFailed("Invalid socket address format".into()))?;

        if !self.streams.contains_key(&dest_addr) {
            let stream = TcpStream::connect(dest_addr)
                .map_err(|e| TransportError::SendFailed(format!("Failed to connect to {}: {}", dest_addr, e)))?;
            let _ = stream.set_nonblocking(true);
            self.streams.insert(dest_addr, stream);
        }

        let frame = crate::transport::types::encode_frame(self.transport_tag(), 0, payload);

        if let Some(stream) = self.streams.get_mut(&dest_addr) {
            stream.write_all(&frame)
                .map_err(|e| TransportError::SendFailed(format!("TCP write failed: {}", e)))?;
            stream.flush()
                .map_err(|e| TransportError::SendFailed(format!("TCP flush failed: {}", e)))?;
            Ok(())
        } else {
            Err(TransportError::SendFailed("Stream not found after connect".into()))
        }
    }

    fn poll_incoming(&mut self) -> Option<TransportPacket> {
        let _ = self.poll_network();
        self.inbox.pop_front()
    }
}
