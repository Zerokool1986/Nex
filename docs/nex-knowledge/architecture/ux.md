# User Experience Architecture & UX Constitution

**Status:** Authoritative Architectural Mapping  
**Source Location:** `NEX/05_UX/NEX-UX-01-CONSTITUTION.md`  

---

## 1. The Fundamental UX Premise

`[DIRECT SOURCE FACT]`
> "NEX is one coherent personal environment, not a collection of unrelated applications. Applications are not siloed destinations; they are specialized lenses viewing the user's unified data, people, and hardware."

---

## 2. The 10 Commandments of NEX UX

`[DIRECT SOURCE FACT]`
1. **Absolute Clarity:** The user must immediately understand what their system is doing and where data lives.
2. **User Sovereignty:** Inviolable user control over data, devices, and connectivity.
3. **Predictability:** Deterministic consequences; no magical background mutations.
4. **Context Over Isolation:** Every object, person, and device is connected to its surrounding relationships.
5. **Progressive Disclosure:** Hide technical complexity until requested. Simplify by staging disclosure, not by removing power.
6. **Local-First by Default:** All features work offline. Connectivity is an enhancement.
7. **Explicit Trust & Capabilities:** Permissions are explicit, human-readable, and bounded.
8. **Consistent UX Grammar:** Universal vocabulary (Person, Identity, Device, Space, Object, Permission, Trust, Sync, Activity).
9. **Universal Accessibility:** Adaptive contrast, screen readers, platform ergonomics.
10. **Expert Power Accessible:** Raw SMT trees, WAL frames, and fuel metering remain accessible.

---

## 3. The 3-Way Separation Rule

`[DIRECT SOURCE FACT]`
```text
+====================================================================================================+
|                                  THE 3-WAY SEPARATION RULE                                         |
+====================================================================================================+
| 1. WHO ARE YOU?            | IDENTITY                                                              |
|                            | Permanent Root ActorID, Ed25519 Keys, Hardware DeviceCertificate.     |
+----------------------------+-----------------------------------------------------------------------+
| 2. WHAT CAN YOU DO?        | PERMISSIONS & AUTHORIZATION ROLES                                     |
|                            | Attenuated CapabilityProof Tokens (Owner, Admin, Member, Guest, Child)|
+----------------------------+-----------------------------------------------------------------------+
| 3. WHAT DO YOU WANT TO SEE?| INTERFACE COMPLEXITY & UX EXPERIENCE LEVEL                            |
|                            | Presentation Filter (Simple, Standard, Advanced, Expert).             |
+====================================================================================================+
```

> [!IMPORTANT]
> **Constitutional Invariant:** Never conflate authorization with interface complexity. The Experience Level controls presentation density only and never modifies cryptographic authority.

---

## 4. The 4-Tier Experience Slider

`[DIRECT SOURCE FACT]` & `[IMPLEMENTATION OBSERVATION]`
In `nex-desktop/src/ui/settings.rs` and `nex-core/src/product/settings.rs`:
- **Simple (🟢):** Minimal view. Routine sync and storage decisions handled automatically.
- **Standard (🔵):** Balanced daily view. Exposes Spaces, sharing controls, device lists, and quotas.
- **Advanced (🟡):** Exposes offline outbox queues, mesh transport preferences, and granular capabilities.
- **Expert (🟣):** Full diagnostic depth: live SMT Merkle proofs, WAL frames, erasure coding matrices, and raw logs.
