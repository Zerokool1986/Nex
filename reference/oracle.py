import json

def serialize_test_vector(category, name, description, inputs, expected_cbor_hex, expected_hash_hex=None, should_reject=False, expected_error_state=None):
    return {
        "category": category,
        "test_name": name,
        "description": description,
        "inputs": inputs,
        "expected_cbor_hex": expected_cbor_hex,
        "expected_hash_hex": expected_hash_hex,
        "expected_error_state": expected_error_state,
        "should_reject": should_reject
    }

conformance_suite = {
    "version": "nex-protocol-v0.8",
    "vectors": []
}

# 1. HashRef Definition & Ordering
conformance_suite["vectors"].append(serialize_test_vector(
    "CBOR",
    "HashRef_Encoding",
    "Validates HashRef is encoded as a 2-element CBOR array [AlgorithmID, Digest]",
    {"alg": 1, "digest": "0000000000000000000000000000000000000000000000000000000000000000"},
    "820158200000000000000000000000000000000000000000000000000000000000000000"
))

# 2. Validation Pipeline Ordering
conformance_suite["vectors"].append(serialize_test_vector(
    "Validation",
    "Pipeline_Lamport_Before_Crypto",
    "Evaluator must reject INVALID_LAMPORT_RANK before evaluating cryptographic signatures.",
    {"parents_lamport": [1], "mutation_lamport": 99, "sig_valid": True},
    None, None,
    should_reject=True,
    expected_error_state="INVALID_LAMPORT_RANK"
))

# 3. Missing History
conformance_suite["vectors"].append(serialize_test_vector(
    "Validation",
    "Missing_History_Unresolved",
    "Missing CausalParents halts evaluation as UNRESOLVED without producing semantic state.",
    {"parents": ["hash_unknown"]},
    None, None,
    should_reject=True,
    expected_error_state="UNRESOLVED"
))

# 4. CRDT Traversal Determinism
conformance_suite["vectors"].append(serialize_test_vector(
    "CRDT",
    "EvaluateCRDT_Traversal_Order",
    "Validates that mutations are evaluated in ascending CausalOrderKey = (LamportRank, MutationID) sequence.",
    {"mutation_a": {"lamport": 2, "id": "FFFF"}, "mutation_b": {"lamport": 2, "id": "0000"}},
    None, None, False,
    expected_error_state="EVAL_ORDER_B_THEN_A" # B wins the lexicographic tiebreaker for earlier processing
))

# 5. Checkpoint Metadata Exclusion
conformance_suite["vectors"].append(serialize_test_vector(
    "CRDT",
    "Checkpoint_Payload_Exclusion",
    "Validates that Type 6 Checkpoint payloads are excluded from EvaluateCRDT state modifications.",
    {"mutation_type": 6, "payload": "frontier_data"},
    None, None, False,
    expected_error_state="STATE_UNCHANGED"
))


with open("conformance_suite_v0.8.json", "w") as f:
    json.dump(conformance_suite, f, indent=4)
print("Conformance suite v0.8 generated.")
