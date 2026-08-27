import asyncio
import time
import sys
import logging
from lab1b_core import Identity, ObjectAuthorityPolicy, AuthenticatedMutation
from engines import EngineDAG, EngineVector

# Logging configuration
logging.basicConfig(level=logging.WARNING, format='%(name)s - %(message)s')
logger = logging.getLogger("StressRunner")

# Global identity registry
registry = {}

def create_identity(name):
    ident = Identity(name)
    registry[ident.pubkey] = ident
    return ident

# Identities
alice = create_identity("Alice")
mallory = create_identity("Mallory") # Attacker

def setup_engines(obj_id, policy):
    dag = EngineDAG(registry)
    vec = EngineVector(registry)
    dag.register_policy(obj_id, policy)
    vec.register_policy(obj_id, policy)
    return dag, vec

async def run_attack_i_flood():
    print("\n--- Running Attack I: Malicious Flood ---")
    obj_id = "doc_flood"
    policy = ObjectAuthorityPolicy({alice.pubkey})
    dag, vec = setup_engines(obj_id, policy)
    
    gen_dag = AuthenticatedMutation(obj_id, alice.pubkey, {"parents": []}, {"val": "genesis"})
    gen_dag.sign(alice)
    dag.add_mutation(gen_dag)
    
    print("Simulating flood of 10,000 invalid mutations...")
    
    start_time = time.perf_counter()
    invalid_mutations = []
    
    # Pre-generate to measure pure rejection cost
    for i in range(10000):
        m = AuthenticatedMutation(obj_id, mallory.pubkey, {"parents": [gen_dag.content_id]}, {"val": "flood"})
        m.sign(mallory)
        invalid_mutations.append(m)
        
    gen_time = time.perf_counter()
    
    dag_rejections = 0
    dag_start = time.perf_counter()
    for m in invalid_mutations:
        if not dag.add_mutation(m):
            dag_rejections += 1
    dag_time = time.perf_counter() - dag_start
    
    vec_rejections = 0
    vec_start = time.perf_counter()
    for m in invalid_mutations:
        if not vec.add_mutation(m):
            vec_rejections += 1
    vec_time = time.perf_counter() - vec_start
    
    print(f"DAG Engine: Rejected {dag_rejections} mutations in {dag_time:.4f}s ({dag_rejections/dag_time:.0f} rejections/sec)")
    print(f"Vector Engine: Rejected {vec_rejections} mutations in {vec_time:.4f}s ({vec_rejections/vec_time:.0f} rejections/sec)")
    
    # Safety Check
    safe = True
    if len(dag.get_heads(obj_id)) != 1: safe = False
    print(f"Safety: {'PASSED' if safe else 'FAILED'}")

async def run_attack_j_byzantine():
    print("\n--- Running Attack J: Byzantine Authorized Node ---")
    obj_id = "doc_byz"
    # Mallory is LEGITIMATELY authorized.
    policy = ObjectAuthorityPolicy({alice.pubkey, mallory.pubkey})
    dag, vec = setup_engines(obj_id, policy)
    
    gen_dag = AuthenticatedMutation(obj_id, alice.pubkey, {"parents": []}, {"val": "genesis"})
    gen_dag.sign(alice)
    dag.add_mutation(gen_dag)
    
    print("Mallory (authorized) generates 5,000 concurrent validly signed forks...")
    
    start_time = time.perf_counter()
    byz_mutations = []
    
    for i in range(5000):
        # Mallory explicitly sets the parent to genesis, creating a massive horizontal fork tree.
        m = AuthenticatedMutation(obj_id, mallory.pubkey, {"parents": [gen_dag.content_id]}, {"val": f"fork_{i}"})
        m.sign(mallory)
        byz_mutations.append(m)
        
    dag_accepted = 0
    dag_start = time.perf_counter()
    for m in byz_mutations:
        if dag.add_mutation(m):
            dag_accepted += 1
    dag_time = time.perf_counter() - dag_start
    
    print(f"DAG Engine: Accepted {dag_accepted} concurrent forks in {dag_time:.4f}s")
    print(f"Memory size of heads: {sys.getsizeof(dag.heads[obj_id])} bytes")
    
    heads = dag.get_heads(obj_id)
    print(f"Safety: FAILED (by definition, attacker controls {len(heads)} valid concurrent states)")
    print("Observation: The engine correctly accepts these because Mallory is authorized. This proves we need mechanisms like revocation, rate limits, or stake to penalize byzantine authorized behavior, because cryptography alone considers these valid.")

async def run_all():
    await run_attack_i_flood()
    await run_attack_j_byzantine()
    # Attack K (Partition + Recovery) requires the ChaosRouter. We will build that in a dedicated network script.

if __name__ == "__main__":
    asyncio.run(run_all())
