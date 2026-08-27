import asyncio
import logging
from core import NodeState, Mutation, ObjectID, ContentID
from router import ChaosRouter

logger = logging.getLogger("Node")

class Node:
    def __init__(self, node_id: str, router: ChaosRouter):
        self.node_id = node_id
        self.router = router
        self.state = NodeState()
        self.queue = router.register_node(node_id)
        self.running = True

    async def start(self):
        """Starts the node's background sync loop."""
        asyncio.create_task(self._process_messages())

    def stop(self):
        self.running = False

    async def _process_messages(self):
        while self.running:
            try:
                # Wait for incoming messages
                msg = await asyncio.wait_for(self.queue.get(), timeout=1.0)
                await self._handle_message(msg.payload)
                self.queue.task_done()
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"Node {self.node_id} error processing message: {e}")

    async def _handle_message(self, payload: dict):
        msg_type = payload.get("type")
        
        if msg_type == "MUTATION":
            mutation_dict = payload.get("mutation")
            if not mutation_dict:
                return
                
            mutation = Mutation.from_dict(mutation_dict)
            
            # Simple signature check (mocked for this lab: if author_id is missing, reject)
            if not mutation.author_id:
                logger.warning(f"Node {self.node_id} rejected unsigned mutation")
                return
                
            # Add to local state
            added = self.state.add_mutation(mutation)
            if added:
                logger.debug(f"Node {self.node_id} accepted mutation {mutation.content_id} for {mutation.obj_id}")
                # Gossip the mutation further
                await self.router.broadcast(self.node_id, payload)
        
        elif msg_type == "SYNC_REQUEST":
            # For simplicity, node just broadcasts its entire state (all mutations)
            # In a real system, this would be optimized with Merkle trees or similar
            for m in self.state.mutations.values():
                await self.router.broadcast(self.node_id, {
                    "type": "MUTATION",
                    "mutation": m.to_dict()
                })

    async def mutate_object(self, obj_id: ObjectID, new_data: dict, is_tombstone: bool = False) -> Mutation:
        """Creates a new mutation and broadcasts it."""
        # Find current heads to use as parents
        parents = list(self.state.heads.get(obj_id, set()))
        
        m = Mutation(
            obj_id=obj_id,
            data=new_data,
            author_id=self.node_id,
            parents=parents,
            is_tombstone=is_tombstone
        )
        
        self.state.add_mutation(m)
        logger.info(f"Node {self.node_id} created mutation {m.content_id} on {obj_id}")
        
        await self.router.broadcast(self.node_id, {
            "type": "MUTATION",
            "mutation": m.to_dict()
        })
        
        return m

    async def trigger_sync(self):
        """Requests state from peers."""
        logger.info(f"Node {self.node_id} triggering network sync")
        await self.router.broadcast(self.node_id, {
            "type": "SYNC_REQUEST"
        })
