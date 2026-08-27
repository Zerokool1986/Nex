import asyncio
import logging
from lab1b_core import Identity, ObjectAuthorityPolicy, AuthenticatedMutation
from engines import EngineDAG, EngineVector

# Logging configuration
logging.basicConfig(level=logging.WARNING, format='%(name)s - %(message)s')
logger = logging.getLogger("TestRunner")

# Global identity registry
registry = {}

def create_identity(name):
    ident = Identity(name)
    registry[ident.pubkey] = ident
    return ident

# Identities
alice = create_identity("Alice")
bob = create_identity("Bob")
mallory = create_identity("Mallory") # Attacker

def setup_engines(obj_id, policy):
    dag = EngineDAG(registry)
    vec = EngineVector(registry)
    dag.register_policy(obj_id, policy)
    vec.register_policy(obj_id, policy)
    return dag, vec

def run_attack_test(attack_name, obj_id, test_fn):
    print(f"\n--- Running {attack_name} ---")
    
    # Alice and Bob share the object.
    policy = ObjectAuthorityPolicy({alice.pubkey, bob.pubkey})
    dag_engine, vec_engine = setup_engines(obj_id, policy)
    
    # Base Genesis State
    gen_dag = AuthenticatedMutation(obj_id, alice.pubkey, {"parents": []}, {"val": "genesis"})
    gen_dag.sign(alice)
    dag_engine.add_mutation(gen_dag)
    
    gen_vec = AuthenticatedMutation(obj_id, alice.pubkey, {"vector": {alice.pubkey: 1}}, {"val": "genesis"})
    gen_vec.sign(alice)
    vec_engine.add_mutation(gen_vec)
    
    # Run test
    try:
        res_dag, res_vec = test_fn(dag_engine, vec_engine, gen_dag, gen_vec, obj_id)
        print(f"DAG Engine Result: {res_dag}")
        print(f"Vector Engine Result: {res_vec}")
        print(f"Metrics (DAG/Vec) - Rejected: {dag_engine.rejected_count}/{vec_engine.rejected_count}, Bytes: {dag_engine.bytes_processed}/{vec_engine.bytes_processed}")
    except Exception as e:
        print(f"Test Failed: {e}")

# Tests
def attack_a_unauthorized(dag, vec, g_dag, g_vec, obj_id):
    # Mallory tries to mutate. She is not in the policy.
    m_dag = AuthenticatedMutation(obj_id, mallory.pubkey, {"parents": [g_dag.content_id]}, {"val": "hacked"})
    m_dag.sign(mallory)
    
    m_vec = AuthenticatedMutation(obj_id, mallory.pubkey, {"vector": {alice.pubkey: 1, mallory.pubkey: 1}}, {"val": "hacked"})
    m_vec.sign(mallory)
    
    dag_acc = dag.add_mutation(m_dag)
    vec_acc = vec.add_mutation(m_vec)
    return f"Accepted: {dag_acc}", f"Accepted: {vec_acc}"

def attack_b_forged(dag, vec, g_dag, g_vec, obj_id):
    # Mallory forges Alice's identity
    m_dag = AuthenticatedMutation(obj_id, alice.pubkey, {"parents": [g_dag.content_id]}, {"val": "hacked"})
    m_dag.sign(mallory) # Signed with Mallory's key, but claims Alice
    
    m_vec = AuthenticatedMutation(obj_id, alice.pubkey, {"vector": {alice.pubkey: 2}}, {"val": "hacked"})
    m_vec.sign(mallory)
    
    dag_acc = dag.add_mutation(m_dag)
    vec_acc = vec.add_mutation(m_vec)
    return f"Accepted: {dag_acc}", f"Accepted: {vec_acc}"

def attack_d_missing_parent(dag, vec, g_dag, g_vec, obj_id):
    # Bob tries to submit a mutation with a parent we don't have
    m_dag = AuthenticatedMutation(obj_id, bob.pubkey, {"parents": ["fake_parent_hash"]}, {"val": "update"})
    m_dag.sign(bob)
    
    m_vec = AuthenticatedMutation(obj_id, bob.pubkey, {"vector": {alice.pubkey: 5, bob.pubkey: 1}}, {"val": "update"})
    m_vec.sign(bob)
    
    dag_acc = dag.add_mutation(m_dag)
    vec_acc = vec.add_mutation(m_vec)
    return f"Accepted: {dag_acc}", f"Accepted: {vec_acc}"

def attack_f_concurrent(dag, vec, g_dag, g_vec, obj_id):
    # Alice and Bob both branch from Genesis offline
    a_dag = AuthenticatedMutation(obj_id, alice.pubkey, {"parents": [g_dag.content_id]}, {"val": "alice_update"})
    a_dag.sign(alice)
    
    b_dag = AuthenticatedMutation(obj_id, bob.pubkey, {"parents": [g_dag.content_id]}, {"val": "bob_update"})
    b_dag.sign(bob)
    
    dag.add_mutation(a_dag)
    dag.add_mutation(b_dag)
    
    a_vec = AuthenticatedMutation(obj_id, alice.pubkey, {"vector": {alice.pubkey: 2}}, {"val": "alice_update"})
    a_vec.sign(alice)
    
    b_vec = AuthenticatedMutation(obj_id, bob.pubkey, {"vector": {alice.pubkey: 1, bob.pubkey: 1}}, {"val": "bob_update"})
    b_vec.sign(bob)
    
    vec.add_mutation(a_vec)
    vec.add_mutation(b_vec)
    
    return f"Heads count: {len(dag.get_heads(obj_id))}", f"Vector state: {vec.vectors.get(obj_id)}"

def run_all():
    run_attack_test("Attack A (Unauthorized)", "docA", attack_a_unauthorized)
    run_attack_test("Attack B (Forged Identity)", "docB", attack_b_forged)
    run_attack_test("Attack D (Missing Parent)", "docD", attack_d_missing_parent)
    run_attack_test("Attack F (Concurrent Writers)", "docF", attack_f_concurrent)

if __name__ == "__main__":
    run_all()
