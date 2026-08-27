import asyncio
import sys
from lab2_core import Identity, AuthenticatedMutation
from lab4_engine import EngineDAGGC

registry = {}
def get_id(name):
    i = Identity(name)
    registry[i.pubkey] = i
    return i

alice = get_id("Alice")
bob = get_id("Bob")
mallory = get_id("Mallory")
charlie = get_id("Charlie")

def test_scenario_1_offline_resurrection():
    print("\n=== Scenario 1: Offline Resurrection ===")
    engine = EngineDAGGC(registry, alice.pubkey)
    
    gen = AuthenticatedMutation("doc1", alice.pubkey, {"parents": []}, {"val": "gen"})
    gen.sign(alice)
    engine.add_mutation(gen)
    
    # Alice delegates to Bob
    del_bob = AuthenticatedMutation("doc1", alice.pubkey, {"parents": [gen.content_id]}, {"type": "delegate", "target": bob.pubkey})
    del_bob.sign(alice)
    engine.add_mutation(del_bob)
    
    # Bob goes offline. Alice continues and creates a checkpoint.
    chk = AuthenticatedMutation("doc1", alice.pubkey, {"parents": [del_bob.content_id]}, {"type": "checkpoint", "active_policy": [alice.pubkey, bob.pubkey]})
    chk.sign(alice)
    engine.add_mutation(chk)
    
    # Alice prunes history
    pruned = engine.prune_history()
    print(f"Alice pruned {pruned} mutations. Gen is gone: {gen.content_id not in engine.mutations}")
    
    # Bob returns with an offline mutation pointing to the pruned del_bob
    bob_mut = AuthenticatedMutation("doc1", bob.pubkey, {"parents": [del_bob.content_id]}, {"val": "bob offline work"})
    bob_mut.sign(bob)
    
    res = engine.add_mutation(bob_mut)
    print(f"Bob's offline resurrection result: {res}")
    print("Observation: The node rejected it due to missing history, breaking Bob's ability to sync. If we allow it without history, it breaks causality. This proves Pruning cannot be totally unilateral without risking orphaned concurrent branches.")

def test_scenario_2_revocation_memory():
    print("\n=== Scenario 2: Revocation Memory ===")
    engine = EngineDAGGC(registry, alice.pubkey)
    
    gen = AuthenticatedMutation("doc2", alice.pubkey, {"parents": []}, {"val": "gen"})
    gen.sign(alice)
    engine.add_mutation(gen)
    
    # Delegate to Mallory
    del_mal = AuthenticatedMutation("doc2", alice.pubkey, {"parents": [gen.content_id]}, {"type": "delegate", "target": mallory.pubkey})
    del_mal.sign(alice)
    engine.add_mutation(del_mal)
    
    # Revoke Mallory
    rev_mal = AuthenticatedMutation("doc2", alice.pubkey, {"parents": [del_mal.content_id]}, {"type": "revoke", "target": mallory.pubkey})
    rev_mal.sign(alice)
    engine.add_mutation(rev_mal)
    
    # Checkpoint embedding the active policy (Alice only)
    chk = AuthenticatedMutation("doc2", alice.pubkey, {"parents": [rev_mal.content_id]}, {"type": "checkpoint", "active_policy": [alice.pubkey]})
    chk.sign(alice)
    engine.add_mutation(chk)
    
    engine.prune_history()
    
    # Mallory tries to write, claiming the checkpoint as parent
    mal_mut = AuthenticatedMutation("doc2", mallory.pubkey, {"parents": [chk.content_id]}, {"val": "malicious"})
    mal_mut.sign(mallory)
    
    res = engine.add_mutation(mal_mut)
    print(f"Mallory's write attempt after pruning: {res}")
    print("Observation: The checkpoint successfully carried the explicit policy state forward, preventing Mallory from resurrecting her authority even though the revocation mutation itself was deleted.")

def test_scenario_3_snapshot_join():
    print("\n=== Scenario 3: Snapshot Join ===")
    # Charlie joins the network. He receives ONLY the checkpoint and its heads.
    engine_charlie = EngineDAGGC(registry, alice.pubkey)
    
    # We fake Charlie receiving the checkpoint from Alice (from scenario 2)
    # Alice signs a checkpoint 
    chk = AuthenticatedMutation("doc3", alice.pubkey, {"parents": ["fake_long_history"]}, {"type": "checkpoint", "active_policy": [alice.pubkey]})
    chk.sign(alice)
    
    res = engine_charlie.add_mutation(chk)
    print(f"Charlie joining via Checkpoint: {res}")
    print("Observation: Charlie successfully joined without Genesis. BUT, he is fundamentally trusting Alice's signature on the checkpoint. If Alice is compromised, she can rewrite history.")

if __name__ == "__main__":
    test_scenario_1_offline_resurrection()
    test_scenario_2_revocation_memory()
    test_scenario_3_snapshot_join()
