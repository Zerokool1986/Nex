import json
import hashlib
import re
from normative_model import NexNormativeModel

def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()

def to_hash_ref(digest: bytes) -> bytes:
    return NexNormativeModel.create_hash_ref(1, digest)

def compute_op_tag(mutation_id_hex: str, op_index: int, op_body: dict) -> bytes:
    m_id = bytes.fromhex(mutation_id_hex)
    body_encoded = NexNormativeModel.nex_cbor_encode(op_body)
    data = NexNormativeModel.nex_cbor_encode([m_id, op_index, body_encoded])
    return to_hash_ref(sha256(b"NEX/OPTAG/v1" + data))

def update_wire_spec():
    spec_path = r"C:\Users\Admin\.gemini\antigravity\brain\1000b28c-b231-4f08-bfa4-ae8b1c1eec73\NEX-PROTOCOL-WIRE-SPEC-v1.1.md"
    try:
        with open(spec_path, "r", encoding="utf-8") as f:
            content = f.read()
        
        # 1. Fix Section 5 Epoch Derivation
        content = re.sub(
            r"Epoch:\s*Parents\s*=\s*∅\s*→\s*0\.\s*Ordinary:\s*max\(P\.Epoch\)\.\s*Tombstone/Resurrection:\s*max\(P\.Epoch\)\s*\+\s*1\.",
            "Epoch:\n    Parents = ∅ → 0\n    M contains at least one RESURRECT → max(P.Epoch) + 1\n    Otherwise → max(P.Epoch)",
            content
        )
        
        # 2. Fix Section 8 EvaluationItem Sorting
        content = re.sub(
            r"3\.\s*Sort all remaining valid operations by `\(Epoch ASC, LamportRank ASC, MutationID ASC, OperationIndex ASC\)`\.",
            "3. Expand operations into `EvaluationItem = { Epoch, LamportRank, MutationID, OperationIndex, OperationBody }`.\n   - Sort `EvaluationItem` records by `(Epoch ASC, LamportRank ASC, MutationID ASC, OperationIndex ASC)`.",
            content
        )
        
        with open(spec_path, "w", encoding="utf-8") as f:
            f.write(content)
        print("Wire Spec updated successfully.")
    except Exception as e:
        print(f"Spec update failed: {e}")

def generate_authoritative_crdt_suite():
    suite = {
        "version": "nex-protocol-v1.1-crdt-authoritative-final",
        "description": "Final CRDT evaluations proving Mechanical Semantics, Object-Global Epoch Reset Boundaries, and Highly Adversarial Late-Arrivals.",
        "vectors": []
    }

    m_id_A_hex = "82015820" + "11" * 32
    m_id_R1_hex = "82015820" + "22" * 32
    m_id_A1_hex = "82015820" + "33" * 32
    m_id_L0_hex = "82015820" + "44" * 32
    m_id_R2_hex = "82015820" + "55" * 32

    # 1. Epoch 0 -> Epoch 1 -> Late Epoch 0 -> Epoch 1 -> Epoch 2 Wipe
    suite["vectors"].append({
        "category": "Mechanical_Semantics",
        "test_name": "Adversarial_Late_Arrival_Epoch_Wipes",
        "description": "Late Epoch 0 Remove(K) evaluates behind Epoch 1 boundary and is annihilated. Subsequent R2 completely wipes Epoch 1.",
        "inputs": {
            "operations": [
                {"id": "A0", "type": "ADD", "key": "K", "payload": "A", "lamport": 1, "epoch": 0, "mutation_id": m_id_A_hex, "op_index": 0},
                {"id": "R1", "type": "RESURRECT", "lamport": 2, "epoch": 1, "mutation_id": m_id_R1_hex, "op_index": 0},
                {"id": "A1", "type": "ADD", "key": "X", "payload": "B", "lamport": 3, "epoch": 1, "mutation_id": m_id_A1_hex, "op_index": 0},
                {"id": "L0", "type": "REMOVE", "key": "K", "lamport": 4, "epoch": 0, "mutation_id": m_id_L0_hex, "op_index": 0}, # Late arrival for Epoch 0
                {"id": "R2", "type": "RESURRECT", "lamport": 5, "epoch": 2, "mutation_id": m_id_R2_hex, "op_index": 0} # Wipes everything before it
            ]
        },
        "expected_state_projection": {
            "AddsMap": {},
            "TombstonesArr": []
        }
    })

    with open("conformance_suite_v1.1_crdt_authoritative.json", "w") as f:
        json.dump(suite, f, indent=4)
    print("Adversarial authoritative mechanical closure vectors generated.")

if __name__ == "__main__":
    update_wire_spec()
    generate_authoritative_crdt_suite()
