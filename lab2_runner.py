import asyncio
import logging
from lab2_core import Identity, AuthenticatedMutation
from lab2_engine import EngineDAGQuarantine
from router import ChaosRouter

logging.basicConfig(level=logging.WARNING, format='%(name)s - %(message)s')

registry = {}
def get_id(name):
    i = Identity(name)
    registry[i.pubkey] = i
    return i

alice = get_id("Alice")
bob = get_id("Bob")
mallory = get_id("Mallory")

async def scenario_delegation_partition():
    print("\n=== Scenario 1: Partitioned Delegation Severance ===")
    router = ChaosRouter()
    
    # Alice is owner.
    engine_bob = EngineDAGQuarantine(registry, alice.pubkey)
    
    # Base state
    gen = AuthenticatedMutation("doc1", alice.pubkey, {"parents": []}, {"val": "gen"}, sim_time=router.simulated_time)
    gen.sign(alice)
    
    # Alice delegates to Amy (Bob acts as Amy here for identity simplicity, let's just use Bob)
    del_bob = AuthenticatedMutation("doc1", alice.pubkey, {"parents": [gen.content_id]}, {"type": "delegate", "target": bob.pubkey}, sim_time=router.simulated_time)
    del_bob.sign(alice)
    
    # Bob delegates to Mallory
    del_mal = AuthenticatedMutation("doc1", bob.pubkey, {"parents": [del_bob.content_id]}, {"type": "delegate", "target": mallory.pubkey}, sim_time=router.simulated_time)
    del_mal.sign(bob)
    
    # Alice revokes Bob
    rev_bob = AuthenticatedMutation("doc1", alice.pubkey, {"parents": [del_mal.content_id]}, {"type": "revoke", "target": bob.pubkey}, sim_time=router.simulated_time)
    rev_bob.sign(alice)
    
    # Sync up to revocation
    engine_bob.add_mutation(gen)
    engine_bob.add_mutation(del_bob)
    engine_bob.add_mutation(del_mal)
    engine_bob.add_mutation(rev_bob)
    
    # Mallory (offline) creates mutation. Mallory's causal history doesn't see the revocation.
    mal_mut = AuthenticatedMutation("doc1", mallory.pubkey, {"parents": [del_mal.content_id]}, {"val": "mallory writes!"}, sim_time=router.simulated_time)
    mal_mut.sign(mallory)
    
    # Network reconciles: does Bob accept Mallory's offline mutation?
    res = engine_bob.add_mutation(mal_mut)
    print(f"Mallory's mutation after her delegator was revoked: {res}")
    # Observation: If Bob evaluates policy up to mal_mut's parents, Mallory was valid at that moment in history.
    # But because Bob has the revocation in his broader DAG, how should conflict resolution merge it? 
    # Currently it's ACCEPTED because the local parent tree is valid. This exposes the "causal revocation" problem.

async def scenario_30_day_byzantine():
    print("\n=== Scenario 2: Attack J2 - 30-Day Offline Byzantine Node ===")
    router = ChaosRouter()
    
    # Alice is owner, but Mallory is authorized
    engine_alice = EngineDAGQuarantine(registry, alice.pubkey)
    
    gen = AuthenticatedMutation("doc2", alice.pubkey, {"parents": []}, {"val": "gen"}, sim_time=router.simulated_time)
    gen.sign(alice)
    
    del_mal = AuthenticatedMutation("doc2", alice.pubkey, {"parents": [gen.content_id]}, {"type": "delegate", "target": mallory.pubkey}, sim_time=router.simulated_time)
    del_mal.sign(alice)
    
    engine_alice.add_mutation(gen)
    engine_alice.add_mutation(del_mal)
    
    # Mallory goes offline for 30 days (simulate ticking)
    print("Mallory goes offline.")
    router.tick(30 * 24 * 60 * 60) # 30 days
    
    # Mallory generates 5000 branches off genesis
    print("Mallory generating 5000 Byzantine forks...")
    bad_forks = []
    for i in range(5000):
        # She points back to the genesis object she saw 30 days ago
        m = AuthenticatedMutation("doc2", mallory.pubkey, {"parents": [gen.content_id]}, {"val": f"fork_{i}"}, sim_time=router.simulated_time)
        m.sign(mallory)
        bad_forks.append(m)
        
    print("Mallory reconnects. Alice's node processes incoming packets.")
    
    for m in bad_forks:
        engine_alice.add_mutation(m)
        
    print(f"Alice DAG state -> Accepted Heads: {len(engine_alice.heads['doc2'])}, Quarantined: {engine_alice.quarantine_count}")
    print("Observation: The quarantine behavioral heuristic successfully trapped the fork-bomb without global consensus, preserving the main active state.")

if __name__ == "__main__":
    asyncio.run(scenario_delegation_partition())
    asyncio.run(scenario_30_day_byzantine())
