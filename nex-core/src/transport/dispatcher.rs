use std::collections::BTreeMap;
use crate::transport::types::{TransportPacket, TransportError};
use crate::transport::adapter::TransportAdapter;

pub struct MultiTransportDispatcher {
    pub adapters: BTreeMap<u16, Box<dyn TransportAdapter>>,
}

impl Default for MultiTransportDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiTransportDispatcher {
    pub fn new() -> Self {
        Self { adapters: BTreeMap::new() }
    }

    pub fn register_adapter(&mut self, adapter: Box<dyn TransportAdapter>) {
        self.adapters.insert(adapter.transport_tag(), adapter);
    }

    /// Dispatches a payload over the target transport, with optional automatic failover to alternative connected carriers
    pub fn dispatch(
        &mut self,
        target_tag: u16,
        destination: &[u8],
        payload: &[u8],
        allow_failover: bool,
    ) -> Result<u16, TransportError> {
        // 1. Try target transport first
        if let Some(target_adapter) = self.adapters.get_mut(&target_tag) {
            if target_adapter.is_connected() {
                if let Ok(()) = target_adapter.send(destination, payload) {
                    return Ok(target_tag);
                }
            }
        }

        // 2. Failover to alternative connected adapters if permitted
        if allow_failover {
            for (&tag, adapter) in self.adapters.iter_mut() {
                if tag != target_tag && adapter.is_connected() {
                    if let Ok(()) = adapter.send(destination, payload) {
                        return Ok(tag); // Successfully failed over
                    }
                }
            }
        }

        Err(TransportError::NoRoutableTransport)
    }

    /// Polls all active adapters for incoming packets
    pub fn poll_all_incoming(&mut self) -> Vec<TransportPacket> {
        let mut packets = Vec::new();
        for adapter in self.adapters.values_mut() {
            while let Some(packet) = adapter.poll_incoming() {
                packets.push(packet);
            }
        }
        packets
    }
}
