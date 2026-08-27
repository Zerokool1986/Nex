import asyncio
import logging
from core import ObjectID, ContentID, Mutation
from router import ChaosRouter
from node import Node

# Configure logging for the test
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger("NastyTest")

async def run_nasty_test():
    router = ChaosRouter()
    
    # Phase 1: Setup honest nodes
    node_a = Node("A", router)
    node_b = Node("B", router)
    node_c = Node("C", router)
    
    await node_a.start()
    await node_b.start()
    await node_c.start()
    
    obj_id = ObjectID("doc_123")
    
    # Initial state sync
    await node_a.mutate_object(obj_id, {"val": "initial_state"})
    await asyncio.sleep(0.5) # Let it propagate
    
    logger.info(f"Node A state: {node_a.state.get_object_state(obj_id)}")
    logger.info(f"Node B state: {node_b.state.get_object_state(obj_id)}")
    logger.info(f"Node C state: {node_c.state.get_object_state(obj_id)}")

    # Phase 2: Disconnected Divergence
    logger.info("=== DISCONNECTING NODES ===")
    router.set_offline("A")
    router.set_offline("B")
    router.set_offline("C")
    
    logger.info("Node A making 50 mutations...")
    for i in range(50):
        await node_a.mutate_object(obj_id, {"val": f"A_mut_{i}"})
        
    logger.info("Node B making 20 mutations and deleting...")
    for i in range(20):
        await node_b.mutate_object(obj_id, {"val": f"B_mut_{i}"})
    await node_b.mutate_object(obj_id, {}, is_tombstone=True) # Deletion
    
    logger.info("Node C making 10 mutations...")
    for i in range(10):
        await node_c.mutate_object(obj_id, {"val": f"C_mut_{i}"})

    # Phase 3: Adversarial Reconnection
    logger.info("=== RECONNECTING WITH CHAOS AND ADVERSARIES ===")
    router.set_chaos(packet_loss_rate=0.4, latency_min_ms=10, latency_max_ms=200)
    
    # Spawn 100 adversarial nodes
    adversaries = []
    for i in range(100):
        adv = Node(f"Adv_{i}", router)
        await adv.start()
        adversaries.append(adv)
        
    router.set_online("A")
    router.set_online("B")
    router.set_online("C")
    
    # Adversaries flood with bad data
    logger.info("Adversaries broadcasting junk...")
    for adv in adversaries:
        asyncio.create_task(adv.mutate_object(obj_id, {"val": "junk"}))
    
    # Honest nodes try to sync
    await node_a.trigger_sync()
    await node_b.trigger_sync()
    await node_c.trigger_sync()
    
    logger.info("Waiting for convergence (5 seconds)...")
    await asyncio.sleep(5)
    
    # Check convergence
    state_a = node_a.state.get_object_state(obj_id)
    state_b = node_b.state.get_object_state(obj_id)
    state_c = node_c.state.get_object_state(obj_id)
    
    logger.info(f"Final State A: {state_a}")
    logger.info(f"Final State B: {state_b}")
    logger.info(f"Final State C: {state_c}")
    
    if state_a == state_b == state_c:
        if state_a is None:
            logger.info("✅ SUCCESS: All nodes converged on Deletion (Tombstone takes precedence)")
        else:
            logger.warning("❌ FAILED: Nodes converged, but tombstone was lost.")
    else:
        logger.error("❌ FAILED: Nodes did not converge.")

    # Cleanup
    node_a.stop()
    node_b.stop()
    node_c.stop()
    for adv in adversaries:
        adv.stop()

if __name__ == "__main__":
    asyncio.run(run_nasty_test())
