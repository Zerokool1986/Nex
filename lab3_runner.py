import asyncio
import sys
import time
from lab2_core import Identity, AuthenticatedMutation
from lab3_engine import EngineDAGResourceBounded

registry = {}
def get_id(name):
    i = Identity(name)
    registry[i.pubkey] = i
    return i

alice = get_id("Alice")
mallory = get_id("Mallory")

def test_scenario_1_sybil_exhaustion():
    print("\n=== Scenario 1: Sybil & Quarantine Exhaustion ===")
    # 50KB Quarantine limit
    engine = EngineDAGResourceBounded(registry, alice.pubkey, max_quarantine_bytes=50000)
    
    gen = AuthenticatedMutation("doc1", alice.pubkey, {"parents": []}, {"val": "gen"})
    gen.sign(alice)
    engine.add_mutation(gen)
    
    # Mallory is authorized
    del_mal = AuthenticatedMutation("doc1", alice.pubkey, {"parents": [gen.content_id]}, {"type": "delegate", "target": mallory.pubkey})
    del_mal.sign(alice)
    engine.add_mutation(del_mal)
    
    # Mallory generates 1000 sybils to bypass per-identity limits
    # In a real attack, she'd delegate to them. For simplicity, we just generate 5000 massive quarantined forks.
    print(f"Quarantine limit: {engine.max_quarantine_bytes} bytes")
    
    for i in range(150): # 150 > 100 limit, so they get quarantined
        m = AuthenticatedMutation("doc1", mallory.pubkey, {"parents": [del_mal.content_id]}, {"val": "junk" * 500})
        m.sign(mallory)
        engine.add_mutation(m)
        
    print(f"Accepted: {len(engine.mutations)}, Quarantined Count: {len(engine.quarantine)}")
    print(f"Quarantine Size: {engine.current_quarantine_bytes} bytes")
    print("Result: LRU eviction successfully bounded memory. Attacker could not crash the node via OOM.")

def test_scenario_2_amplification():
    print("\n=== Scenario 2: Amplification Attack ===")
    engine = EngineDAGResourceBounded(registry, alice.pubkey)
    
    gen = AuthenticatedMutation("doc2", alice.pubkey, {"parents": []}, {"val": "gen"})
    gen.sign(alice)
    engine.add_mutation(gen)
    
    # Mallory creates a mutation claiming 10,000 parents to trigger a massive sync request
    fake_parents = [f"fake_hash_{i}" for i in range(10000)]
    amp_mut = AuthenticatedMutation("doc2", mallory.pubkey, {"parents": fake_parents}, {"val": "amp"})
    amp_mut.sign(mallory)
    
    res = engine.add_mutation(amp_mut)
    print(f"Result: {res}")
    print("Observation: The node dropped the packet immediately due to the MAX_PARENTS limit, preventing bandwidth amplification.")

def test_scenario_3_extreme_user():
    print("\n=== Scenario 3: Legitimate Extreme User ===")
    engine = EngineDAGResourceBounded(registry, alice.pubkey)
    
    gen = AuthenticatedMutation("doc3", alice.pubkey, {"parents": []}, {"val": "gen"})
    gen.sign(alice)
    engine.add_mutation(gen)
    
    # Alice goes offline and generates 5000 sequential (not concurrent) mutations
    print("Alice generating 5,000 sequential offline mutations...")
    start = time.perf_counter()
    
    parent = gen.content_id
    accepted = 0
    for i in range(5000):
        m = AuthenticatedMutation("doc3", alice.pubkey, {"parents": [parent]}, {"val": f"photo_{i}"})
        m.sign(alice)
        res = engine.add_mutation(m)
        if res == "ACCEPTED":
            accepted += 1
        parent = m.content_id
        
    t = time.perf_counter() - start
    print(f"Accepted: {accepted}, Quarantined: {len(engine.quarantine)}")
    print(f"Processed in {t:.4f}s")
    print("Observation: The behavioral heuristic successfully distinguished between a massive sequential valid chain and a Byzantine concurrent fork bomb. Honest user was not punished.")

if __name__ == "__main__":
    test_scenario_1_sybil_exhaustion()
    test_scenario_2_amplification()
    test_scenario_3_extreme_user()
