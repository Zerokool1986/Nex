import logging
from typing import Dict, Set, List
from lab2_core import AuthenticatedMutation, PolicyEngine

logger = logging.getLogger("Lab2Engine")

class EngineDAGQuarantine:
    """DAG Engine that implements behavioral containment via Quarantine."""
    def __init__(self, identity_registry: dict, genesis_owner: str):
        self.mutations: Dict[str, AuthenticatedMutation] = {}
        self.identity_registry = identity_registry
        self.policy_engine = PolicyEngine(genesis_owner)
        
        self.heads: Dict[str, Set[str]] = {}
        
        # Quarantine Buffer
        self.quarantine: Dict[str, AuthenticatedMutation] = {}
        
        self.rejected_count = 0
        self.quarantine_count = 0

    def add_mutation(self, mutation: AuthenticatedMutation) -> str:
        """Returns 'ACCEPTED', 'QUARANTINED', or 'REJECTED'."""
        if mutation.content_id in self.mutations or mutation.content_id in self.quarantine:
            return "DUPLICATE"
            
        # 1. Check Authenticity (Crypto)
        if not mutation.verify_signature(self.identity_registry):
            self.rejected_count += 1
            return "REJECTED"
            
        parents = mutation.causal_metadata.get("parents", [])
        
        # Check Causality
        for p in parents:
            if p not in self.mutations:
                # If a parent is in quarantine, this mutation is structurally valid but inherits the quarantine.
                if p in self.quarantine:
                    self.quarantine[mutation.content_id] = mutation
                    self.quarantine_count += 1
                    return "QUARANTINED"
                else:
                    self.rejected_count += 1
                    return "REJECTED"

        # 2. Check Authority (Dynamic Policy)
        # We evaluate the policy up to the *parents* of this mutation to see if the author was authorized at that causal branch.
        # This prevents an actor from retroactively authorizing themselves.
        authorized_keys = self.policy_engine.evaluate(self.mutations, set(parents))
        
        if mutation.author_pubkey not in authorized_keys:
            self.rejected_count += 1
            return "REJECTED"
            
        # 3. Check Behavior (Containment)
        # Behavioral Heuristic: Rate limiting branch creation. 
        # If an author tries to create > 100 concurrent forks from the exact same parent, quarantine it.
        # Note: In a real system, we'd look at the global DAG width for this author.
        if len(parents) == 1:
            parent = parents[0]
            forks_from_parent = sum(1 for m in self.mutations.values() if parent in m.causal_metadata.get("parents", []) and m.author_pubkey == mutation.author_pubkey)
            if forks_from_parent > 100:
                self.quarantine[mutation.content_id] = mutation
                self.quarantine_count += 1
                return "QUARANTINED"

        # Accepted
        self.mutations[mutation.content_id] = mutation
        if mutation.obj_id not in self.heads:
            self.heads[mutation.obj_id] = set()
            
        for p in parents:
            if p in self.heads[mutation.obj_id]:
                self.heads[mutation.obj_id].remove(p)
                
        self.heads[mutation.obj_id].add(mutation.content_id)
        return "ACCEPTED"

    def get_heads(self, obj_id: str) -> List[str]:
        return list(self.heads.get(obj_id, set()))
