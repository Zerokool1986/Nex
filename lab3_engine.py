import logging
import sys
from collections import OrderedDict
from typing import Dict, Set, List
from lab2_core import AuthenticatedMutation, PolicyEngine

logger = logging.getLogger("Lab3Engine")

class ResourceExhaustedError(Exception):
    pass

class EngineDAGResourceBounded:
    """DAG Engine with hard resource limits and LRU Quarantine."""
    def __init__(self, identity_registry: dict, genesis_owner: str, max_quarantine_bytes: int = 1000000): # Default 1MB quarantine
        self.mutations: Dict[str, AuthenticatedMutation] = {}
        self.identity_registry = identity_registry
        self.policy_engine = PolicyEngine(genesis_owner)
        self.heads: Dict[str, Set[str]] = {}
        
        # Resource bounds
        self.max_quarantine_bytes = max_quarantine_bytes
        self.current_quarantine_bytes = 0
        self.quarantine: OrderedDict[str, AuthenticatedMutation] = OrderedDict() # OrderedDict for LRU
        
        # Amplification limits
        self.MAX_PARENTS_PER_MUTATION = 50 

    def _evict_quarantine(self):
        """LRU eviction of quarantine buffer until under limit."""
        while self.current_quarantine_bytes > self.max_quarantine_bytes and self.quarantine:
            # popitem(last=False) pops the first inserted (least recently used in this context)
            k, v = self.quarantine.popitem(last=False)
            self.current_quarantine_bytes -= sys.getsizeof(str(v.to_dict()))

    def _add_to_quarantine(self, mutation: AuthenticatedMutation):
        mut_size = sys.getsizeof(str(mutation.to_dict()))
        self.quarantine[mutation.content_id] = mutation
        self.quarantine.move_to_end(mutation.content_id) # Mark as recently used
        self.current_quarantine_bytes += mut_size
        self._evict_quarantine()

    def add_mutation(self, mutation: AuthenticatedMutation) -> str:
        """Returns 'ACCEPTED', 'QUARANTINED', or 'REJECTED'."""
        if mutation.content_id in self.mutations or mutation.content_id in self.quarantine:
            return "DUPLICATE"
            
        # 1. Authenticity
        if not mutation.verify_signature(self.identity_registry):
            return "REJECTED"
            
        parents = mutation.causal_metadata.get("parents", [])
        
        # Resource Attack Defense: Amplification limit
        if len(parents) > self.MAX_PARENTS_PER_MUTATION:
            logger.warning(f"Amplification defense: Rejected mutation with {len(parents)} parents.")
            return "REJECTED"

        # Check Causality
        for p in parents:
            if p not in self.mutations:
                if p in self.quarantine:
                    self._add_to_quarantine(mutation)
                    return "QUARANTINED"
                else:
                    return "REJECTED" # Missing history

        # 2. Authority (Policy Engine)
        authorized_keys = self.policy_engine.evaluate(self.mutations, set(parents))
        
        if mutation.author_pubkey not in authorized_keys:
            return "REJECTED"
            
        # 3. Behavior (Data vs Authority semantics)
        is_authority_transition = mutation.data.get("type") in ["delegate", "revoke"]
        
        # Extreme User Heuristic: 
        # If this is a data transition and there's a massive sequential chain, that's fine (it's just a lot of work).
        # But if it's a massive concurrent horizontal fork bomb, quarantine it.
        # For this lab, we check horizontal forks (concurrent branches from same parent).
        if len(parents) == 1:
            parent = parents[0]
            forks_from_parent = sum(1 for m in self.mutations.values() if parent in m.causal_metadata.get("parents", []) and m.author_pubkey == mutation.author_pubkey)
            
            # Stricter heuristic for authority transitions to prevent Sybil attacks
            if is_authority_transition and forks_from_parent > 10:
                self._add_to_quarantine(mutation)
                return "QUARANTINED"
            # Looser heuristic for data to allow legitimate extreme users (e.g. they created 10,000 photos offline sequentially)
            elif not is_authority_transition and forks_from_parent > 100:
                self._add_to_quarantine(mutation)
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
