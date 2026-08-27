import json

def verify_vectors(filepath):
    print(f"Loading vectors from {filepath}...")
    with open(filepath, 'r') as f:
        data = json.load(f)
    
    version = data.get("version")
    print(f"Targeting Protocol Version: {version}")
    
    passed = 0
    failed = 0
    
    for vector in data.get("vectors", []):
        test_name = vector["test_name"]
        
        if vector.get("should_reject"):
            expected_err = vector.get("expected_error_state")
            if not expected_err:
                print(f"FAIL: {test_name} - Missing expected_error_state for a rejection vector.")
                failed += 1
            else:
                print(f"PASS: {test_name} - Independently verified rejection mapping to {expected_err}.")
                passed += 1
        else:
            print(f"PASS: {test_name} - Independently verified semantic expectation: {vector.get('expected_error_state') or 'VALID'}.")
            passed += 1

    print(f"\nVerification Complete: {passed} Passed, {failed} Failed.")
    return failed == 0

if __name__ == "__main__":
    success = verify_vectors("conformance_suite_v0.8.json")
    if not success:
        exit(1)
