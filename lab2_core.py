import hashlib
import json
import uuid
from typing import List, Dict, Optional, Set, Any
import copy

class Identity:
    def __init__(self, name: str):
        self.pubkey = f"pub_{name}_{uuid.uuid4().hex[:8]}"
        self.privkey = f"priv_{name}"

    def sign(self, payload: bytes) -> str:
        return hashlib.sha256(self.privkey.encode() + payload).hexdigest()

    def verify(self, signature: str, payload: bytes) -> bool:
        expected = hashlib.sha256(self.privkey.encode() + payload).hexdigest()
        return signature == expected

class AuthenticatedMutation:
    def __init__(self, obj_id: str, author_pubkey: str, causal_metadata: dict, data: dict, is_tombstone: bool = False, sim_time: float = 0.0):
        self.obj_id = obj_id
        self.author_pubkey = author_pubkey
        self.causal_metadata = causal_metadata
        self.data = data
        self.is_tombstone = is_tombstone
        self.timestamp = sim_time # Use logical clock
        self.signature = ""
        self.content_id = ""

    def sign(self, identity: Identity):
        payload = self._serialize_for_signing()
        self.signature = identity.sign(payload)
        self.content_id = hashlib.sha256(self.signature.encode()).hexdigest()

    def _serialize_for_signing(self) -> bytes:
        payload = {
            "obj_id": self.obj_id,
            "author": self.author_pubkey,
            "causal": self.causal_metadata,
            "data": self.data,
            "tombstone": self.is_tombstone,
            "ts": self.timestamp
        }
        return json.dumps(payload, sort_keys=True).encode('utf-8')

    def verify_signature(self, identity_registry: dict) -> bool:
        if self.author_pubkey not in identity_registry: return False
        return identity_registry[self.author_pubkey].verify(self.signature, self._serialize_for_signing())

class PolicyEngine:
    """Evaluates DAG history to determine the active authority policy."""
    def __init__(self, genesis_owner: str):
        self.genesis_owner = genesis_owner
        
    def evaluate(self, mutations: Dict[str, AuthenticatedMutation], head_ids: Set[str]) -> Set[str]:
        """Returns the set of currently authorized public keys based on the causal DAG."""
        authorized = {self.genesis_owner}
        
        # Traverse the DAG from genesis to heads to apply delegations/revocations
        # In a real CRDT, this would be a topological sort or an event-sourced fold.
        # For this lab, we'll do a simplified chronological replay of policy mutations.
        
        # Filter for policy mutations
        policy_muts = []
        for m in mutations.values():
            if m.data.get("type") in ["delegate", "revoke"]:
                policy_muts.append(m)
                
        # Sort by topological logical clock
        policy_muts.sort(key=lambda x: x.timestamp)
        
        for m in policy_muts:
            if m.data["type"] == "delegate":
                target = m.data.get("target")
                # Only active authorities can delegate
                if target and m.author_pubkey in authorized:
                    authorized.add(target)
            elif m.data["type"] == "revoke":
                target = m.data.get("target")
                # Only genesis owner (or higher capability) can revoke
                if target and target in authorized and m.author_pubkey == self.genesis_owner:
                    authorized.remove(target)
                    
        return authorized
