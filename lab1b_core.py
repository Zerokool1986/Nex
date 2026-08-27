import hashlib
import json
import uuid
import time
from typing import List, Dict, Optional, Set, Any
import copy

class Identity:
    """Mock Ed25519 Identity for the lab."""
    def __init__(self, name: str):
        self.pubkey = f"pub_{name}_{uuid.uuid4().hex[:8]}"
        self.privkey = f"priv_{name}" # In a real system, keep secret

    def sign(self, payload: bytes) -> str:
        # Mock signature: hash of private key + payload
        return hashlib.sha256(self.privkey.encode() + payload).hexdigest()

    def verify(self, signature: str, payload: bytes) -> bool:
        # Mock verification: recompute with assumed privkey 
        # (in this lab we cheat the mock to verify, in reality we use ed25519.verify)
        expected = hashlib.sha256(self.privkey.encode() + payload).hexdigest()
        return signature == expected

class ObjectAuthorityPolicy:
    def __init__(self, authorized_writers: Set[str]):
        self.authorized_writers = authorized_writers
        
    def is_authorized(self, pubkey: str) -> bool:
        return pubkey in self.authorized_writers
        
    def to_dict(self):
        return {"authorized_writers": list(self.authorized_writers)}
        
    @classmethod
    def from_dict(cls, d: dict):
        return cls(set(d.get("authorized_writers", [])))

class AuthenticatedMutation:
    def __init__(self, obj_id: str, author_pubkey: str, causal_metadata: dict, data: dict, is_tombstone: bool = False):
        self.obj_id = obj_id
        self.author_pubkey = author_pubkey
        self.causal_metadata = causal_metadata
        self.data = data
        self.is_tombstone = is_tombstone
        self.timestamp = time.time_ns()
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
        # In a real system, we'd use the pubkey directly. Here we use a registry to lookup the mock privkey for verification.
        if self.author_pubkey not in identity_registry:
            return False
        identity = identity_registry[self.author_pubkey]
        return identity.verify(self.signature, self._serialize_for_signing())

    def to_dict(self):
        return {
            "content_id": self.content_id,
            "obj_id": self.obj_id,
            "author_pubkey": self.author_pubkey,
            "causal_metadata": self.causal_metadata,
            "data": self.data,
            "is_tombstone": self.is_tombstone,
            "timestamp": self.timestamp,
            "signature": self.signature
        }

    @classmethod
    def from_dict(cls, d: dict):
        m = cls(d['obj_id'], d['author_pubkey'], d['causal_metadata'], d['data'], d['is_tombstone'])
        m.timestamp = d['timestamp']
        m.signature = d['signature']
        m.content_id = d['content_id']
        return m
