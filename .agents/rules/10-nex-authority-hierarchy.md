---
description: Formal 8-Level NEX Conflict Resolution and Authority Hierarchy
always_apply: true
---

# NEX Authority Hierarchy & Conflict Resolution Protocol

When any requirement, specification, test, or code snippet appears to conflict, Antigravity MUST resolve the ambiguity by strictly ascending the **Authority Hierarchy**. Lower levels NEVER override higher levels.

```text
+=============================================================================+
|                        NEX AUTHORITY HIERARCHY                              |
+=============================================================================+
| LEVEL 1: NEX CONSTITUTION (NEX-00..05)                                      |
|   - Supreme architectural law: Sovereignty, Local-First, Capabilities      |
+-----------------------------------------------------------------------------+
| LEVEL 2: FROZEN WIRE & PERSISTENCE CONTRACTS (NEX/WIRE/v1, NEX/WAL/v1)      |
|   - Binary wire framing, 48-byte headers, append-only WAL layout            |
+-----------------------------------------------------------------------------+
| LEVEL 3: SEALED ARCHITECTURAL DECISION RECORDS (ADRs)                       |
|   - Frozen decisions, explicitly rejected alternatives & architectural why  |
+-----------------------------------------------------------------------------+
| LEVEL 4: SEALED GATE SPECIFICATIONS (R50-0 .. R65-0)                        |
|   - Subsystem boundaries, mathematical formulas, state machine transitions   |
+-----------------------------------------------------------------------------+
| LEVEL 5: BINDING CONTRACT SUITES & FFI DEFINITIONS                          |
|   - C ABI v1 signatures, DirectByteBuffer JNI layout, streaming CAS headers  |
+-----------------------------------------------------------------------------+
| LEVEL 6: CANONICAL RUST SUBSTRATE IMPLEMENTATION                            |
|   - `crates/nex-core` active engine code and modules                        |
+-----------------------------------------------------------------------------+
| LEVEL 7: AUTHORITATIVE TEST MATRIX (342/342 Tests)                          |
|   - Integration test suites, fuzzers, physical stress harnesses              |
+-----------------------------------------------------------------------------+
| LEVEL 8: EXPERIMENTAL / PROPOSED REALIZATION WORK                           |
|   - Draft specifications, WIP feature branches, unratified suggestions     |
+=============================================================================+
```

## Conflict Resolution Protocol
- If an implementation detail (Level 6) conflicts with a Sealed Gate Spec (Level 4), **the implementation must be corrected to match the specification**.
- If a new feature proposal (Level 8) introduces a central coordinator, **it is rejected immediately under Level 1 (Constitution)**.
- If a wire framing change is proposed, **it is rejected under Level 2 (Frozen Wire Specification)** unless a formal version bump to `NEX/WIRE/v2` is ratified at Level 1.
