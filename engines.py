import logging
from typing import Dict, Set, List
from lab1b_core import AuthenticatedMutation, ObjectAuthorityPolicy

logger = logging.getLogger("Engines")

class BaseEngine:
    def __init__(self, identity_registry: dict):
        self.mutations: Dict[str, AuthenticatedMutation] = {}
        self.identity_registry = identity_registry
        self.authority_policies: Dict[str, ObjectAuthorityPolicy] = {}
        
        # Metrics
        self.bytes_processed = 0
        self.rejected_count = 0

    def register_policy(self, obj_id: str, policy: ObjectAuthorityPolicy):
        self.authority_policies[obj_id] = policy

    def _validate_auth(self, mutation: AuthenticatedMutation) -> bool:
        # 1. Check cryptographic signature (Identity)
        if not mutation.verify_signature(self.identity_registry):
            logger.warning(f"Rejected: Invalid signature on {mutation.content_id}")
            self.rejected_count += 1
            return False
            
        # 2. Check authority policy (Authority)
        policy = self.authority_policies.get(mutation.obj_id)
        if policy and not policy.is_authorized(mutation.author_pubkey):
            logger.warning(f"Rejected: Unauthorized writer {mutation.author_pubkey} for {mutation.obj_id}")
            self.rejected_count += 1
            return False
            
        return True

class EngineDAG(BaseEngine):
    def __init__(self, identity_registry: dict):
        super().__init__(identity_registry)
        self.heads: Dict[str, Set[str]] = {} # obj_id -> set of ContentIDs

    def add_mutation(self, mutation: AuthenticatedMutation) -> bool:
        self.bytes_processed += len(str(mutation.to_dict()))
        
        if mutation.content_id in self.mutations:
            return False # Already have it
            
        if not self._validate_auth(mutation):
            return False

        parents = mutation.causal_metadata.get("parents", [])
        
        # 3. Check Causality (DAG)
        for p in parents:
            if p not in self.mutations:
                # Missing parent! Quarantine or reject. For this lab, reject (simulate missing history)
                logger.warning(f"Rejected: Missing parent {p} for {mutation.content_id}")
                self.rejected_count += 1
                return False

        # Add to state
        self.mutations[mutation.content_id] = mutation
        if mutation.obj_id not in self.heads:
            self.heads[mutation.obj_id] = set()
            
        for p in parents:
            if p in self.heads[mutation.obj_id]:
                self.heads[mutation.obj_id].remove(p)
                
        self.heads[mutation.obj_id].add(mutation.content_id)
        return True

    def get_heads(self, obj_id: str) -> List[str]:
        return list(self.heads.get(obj_id, set()))

class EngineVector(BaseEngine):
    def __init__(self, identity_registry: dict):
        super().__init__(identity_registry)
        self.vectors: Dict[str, Dict[str, int]] = {} # obj_id -> {pubkey -> seq}

    def add_mutation(self, mutation: AuthenticatedMutation) -> bool:
        self.bytes_processed += len(str(mutation.to_dict()))
        
        if mutation.content_id in self.mutations:
            return False
            
        if not self._validate_auth(mutation):
            return False

        vector = mutation.causal_metadata.get("vector", {})
        obj_vector = self.vectors.get(mutation.obj_id, {})

        # 3. Check Causality (Vector Clock)
        # Check if this mutation's causal past is satisfied by our current state
        author = mutation.author_pubkey
        author_seq = vector.get(author, 1)
        
        # Simplistic check: If the vector claims an author is at seq 5, but we only have seq 3, 
        # it means we are missing intermediate states.
        # Actually, standard vector clock merge allows concurrent updates but rejects if causality is broken.
        for k, v in vector.items():
            if k != author and obj_vector.get(k, 0) < v:
                logger.warning(f"Rejected: Vector clock implies missing history from {k} (need {v}, have {obj_vector.get(k, 0)})")
                self.rejected_count += 1
                return False

        # Accept mutation
        self.mutations[mutation.content_id] = mutation
        
        # Update our local vector clock
        if mutation.obj_id not in self.vectors:
            self.vectors[mutation.obj_id] = {}
            
        # Max of current and incoming vector
        for k, v in vector.items():
            self.vectors[mutation.obj_id][k] = max(self.vectors[mutation.obj_id].get(k, 0), v)
            
        return True
