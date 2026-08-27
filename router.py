import asyncio
import random
from typing import Dict, List, Optional
import logging

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger("ChaosRouter")

class Message:
    def __init__(self, sender_id: str, receiver_id: str, payload: dict):
        self.sender_id = sender_id
        self.receiver_id = receiver_id
        self.payload = payload

class ChaosRouter:
    """Simulates a highly chaotic, adversarial network with a logical clock."""
    def __init__(self):
        self.nodes = {}  # node_id -> asyncio.Queue
        self.offline_nodes = set()
        
        # Chaos parameters
        self.packet_loss_rate = 0.0 # 0.0 to 1.0
        self.latency_min_ms = 0
        self.latency_max_ms = 0
        self.running = True
        
        # Logical Simulation Clock (starts at 0)
        self.simulated_time = 0.0

    def tick(self, seconds: float):
        """Advances the simulation clock by the specified number of seconds."""
        self.simulated_time += seconds
        logger.info(f"Simulation Clock advanced by {seconds}s to {self.simulated_time}")

    def register_node(self, node_id: str) -> asyncio.Queue:
        queue = asyncio.Queue()
        self.nodes[node_id] = queue
        return queue

    def set_offline(self, node_id: str):
        self.offline_nodes.add(node_id)
        logger.info(f"Node {node_id} is now OFFLINE")

    def set_online(self, node_id: str):
        if node_id in self.offline_nodes:
            self.offline_nodes.remove(node_id)
        logger.info(f"Node {node_id} is now ONLINE")

    def set_chaos(self, packet_loss_rate: float, latency_min_ms: int, latency_max_ms: int):
        self.packet_loss_rate = packet_loss_rate
        self.latency_min_ms = latency_min_ms
        self.latency_max_ms = latency_max_ms
        logger.info(f"Chaos updated: loss={packet_loss_rate*100}%, latency={latency_min_ms}-{latency_max_ms}ms")

    async def broadcast(self, sender_id: str, payload: dict):
        """Broadcasts a message to all registered nodes (simulating mesh broadcast)"""
        if sender_id in self.offline_nodes:
            return # Offline nodes can't send
            
        for receiver_id in self.nodes.keys():
            if receiver_id == sender_id:
                continue
            # Route individual message to simulate independent path conditions
            asyncio.create_task(self._deliver(Message(sender_id, receiver_id, payload)))

    async def _deliver(self, msg: Message):
        if msg.receiver_id in self.offline_nodes:
            return # Drops packet
            
        if random.random() < self.packet_loss_rate:
            # Packet dropped
            return
            
        # Simulate latency
        if self.latency_max_ms > 0:
            delay = random.uniform(self.latency_min_ms, self.latency_max_ms) / 1000.0
            await asyncio.sleep(delay)
            
        # Re-check offline status after delay
        if msg.receiver_id in self.offline_nodes:
            return
            
        queue = self.nodes.get(msg.receiver_id)
        if queue:
            await queue.put(msg)
