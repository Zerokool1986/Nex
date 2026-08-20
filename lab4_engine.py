import logging
from typing import Dict, Set, List
from lab2_core import AuthenticatedMutation, PolicyEngine

logger = logging.getLogger("Lab4Engine")

class EngineDAGGC:
    """DAG Engine that supports Checkpoints and Historical Pruning."""
    def __init__(self, identity_registry: dict, genesis_owner: str):
        self.mutations: Dict[str, AuthenticatedMutation] = {}
        self.identity_registry = identity_registry
        self.policy_engine = PolicyEngine(genesis_owner)
        self.heads: Dict[str, Set[str]] = {}
        
        # Checkpoint state
        self.active_checkpoint: str = None
        self.checkpoint_policy_state: Set[str] = {genesis_owner}
        
    def add_mutation(self, mutation: AuthenticatedMutation) -> str:
        if mutation.content_id in self.mutations:
            return "DUPLICATE"
            
        if not mutation.verify_signature(self.identity_registry):
            return "REJECTED"
            
        parents = mutation.causal_metadata.get("parents", [])
        
        # GC Support: If parents are missing, check if they are covered by an active checkpoint.
        # If a mutation points to a parent we pruned, but the mutation's logical time is older than the checkpoint,
        # it is obsolete. If it's newer, it's an offline resurrection we need to negotiate.
        missing_parents = [p for p in parents if p not in self.mutations]
        if missing_parents:
            # We don't have the history. 
            if self.active_checkpoint:
                # Naive implementation for simulation: if we have a checkpoint, we accept it as a concurrent branch 
                # (resurrection) if it's signed by someone in the checkpoint policy state.
                pass 
            else:
                return "REJECTED (Missing History)"

        # Authority
        # If the mutation descends from a checkpoint, use the checkpoint's embedded policy state as the base.
        if self.active_checkpoint and self.active_checkpoint in parents:
            authorized_keys = self.checkpoint_policy_state
        else:
            authorized_keys = self.policy_engine.evaluate(self.mutations, set(parents))
            
        if mutation.author_pubkey not in authorized_keys:
            return "REJECTED (Unauthorized)"

        # Accept
        self.mutations[mutation.content_id] = mutation
        if mutation.obj_id not in self.heads:
            self.heads[mutation.obj_id] = set()
            
        for p in parents:
            if p in self.heads[mutation.obj_id]:
                self.heads[mutation.obj_id].remove(p)
                
        self.heads[mutation.obj_id].add(mutation.content_id)
        
        # Handle Checkpoint logic
        if mutation.data.get("type") == "checkpoint":
            self.active_checkpoint = mutation.content_id
            # The checkpoint must explicitly embed the active policy state at that moment
            self.checkpoint_policy_state = set(mutation.data.get("active_policy", []))
            
        return "ACCEPTED"

    def prune_history(self):
        """Drops all mutations that are causal ancestors of the active checkpoint."""
        if not self.active_checkpoint:
            return 0
            
        # For simulation, we'll just keep the checkpoint and the heads. 
        # A real topological prune requires traversing from the checkpoint down to genesis.
        to_keep = {self.active_checkpoint}
        for head_set in self.heads.values():
            to_keep.update(head_set)
            
        pruned_count = 0
        keys = list(self.mutations.keys())
        for k in keys:
            if k not in to_keep:
                del self.mutations[k]
                pruned_count += 1
                
        return pruned_count
