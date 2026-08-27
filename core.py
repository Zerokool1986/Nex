import hashlib
import json
import uuid
import time
from typing import List, Dict, Optional, Set

class ObjectID(str):
    pass

class ContentID(str):
    pass

class Mutation:
    def __init__(self, obj_id: ObjectID, data: dict, author_id: str, parents: List[ContentID], is_tombstone: bool = False):
        self.obj_id = obj_id
        self.data = data
        self.author_id = author_id
        self.parents = sorted(parents) # Sort for determinism in hashing
        self.timestamp = time.time_ns() # Logical/wall clock hybrid for testing
        self.is_tombstone = is_tombstone
        
        # Calculate ContentID (hash of the mutation's immutable properties)
        self.content_id = self._calculate_hash()

    def _calculate_hash(self) -> ContentID:
        # A deterministic JSON serialization of the mutation's state
        payload = {
            "obj_id": self.obj_id,
            "data": self.data,
            "author_id": self.author_id,
            "parents": self.parents,
            "is_tombstone": self.is_tombstone,
            "timestamp": self.timestamp
        }
        serialized = json.dumps(payload, sort_keys=True).encode('utf-8')
        return ContentID(hashlib.sha256(serialized).hexdigest())

    def to_dict(self):
        return {
            "content_id": self.content_id,
            "obj_id": self.obj_id,
            "data": self.data,
            "author_id": self.author_id,
            "parents": self.parents,
            "is_tombstone": self.is_tombstone,
            "timestamp": self.timestamp
        }
    
    @classmethod
    def from_dict(cls, d: dict):
        # Reconstruct mutation from dict, preserving timestamp and content_id
        m = cls(d['obj_id'], d['data'], d['author_id'], d['parents'], d['is_tombstone'])
        m.timestamp = d['timestamp']
        m.content_id = d['content_id']
        return m

class NodeState:
    """Represents the in-memory database of a Node"""
    def __init__(self):
        # Maps ContentID -> Mutation
        self.mutations: Dict[ContentID, Mutation] = {}
        # Maps ObjectID -> Set of 'leaf' ContentIDs (the current tips of the DAG)
        self.heads: Dict[ObjectID, Set[ContentID]] = {}

    def add_mutation(self, mutation: Mutation) -> bool:
        """Adds a mutation to the local DAG. Returns True if added, False if already exists."""
        if mutation.content_id in self.mutations:
            return False
            
        self.mutations[mutation.content_id] = mutation
        
        # Update heads (remove parents from heads, add this new mutation)
        if mutation.obj_id not in self.heads:
            self.heads[mutation.obj_id] = set()
            
        for parent_id in mutation.parents:
            if parent_id in self.heads[mutation.obj_id]:
                self.heads[mutation.obj_id].remove(parent_id)
                
        self.heads[mutation.obj_id].add(mutation.content_id)
        return True

    def get_object_state(self, obj_id: ObjectID) -> dict:
        """Naive CRDT merge logic - Last Writer Wins (LWW) based on timestamp for conflicts.
        If a tombstone is in the heads, it's deleted."""
        if obj_id not in self.heads or not self.heads[obj_id]:
            return None
            
        heads = self.heads[obj_id]
        
        # Check for tombstones in heads
        for h in heads:
            if self.mutations[h].is_tombstone:
                return None # Object is deleted
                
        # Resolve conflicts by LWW (timestamp)
        latest_mutation = max((self.mutations[h] for h in heads), key=lambda m: m.timestamp)
        return latest_mutation.data
