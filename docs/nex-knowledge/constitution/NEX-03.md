# NEX-03: Self-Sovereign Identity & Trust

**Authority:** NEX Supreme Constitutional Law (Level 1)  
**Authoritative Source Location:** `NEX/00_CONSTITUTION/NEX-03_IDENTITY_TRUST.md`  
**Status:** FROZEN & IMMUTABLE  

---

## 1. Constitutional Directives

`[DIRECT SOURCE FACT]`
1. **Actor IDs:**
   An `ActorID` is the cryptographic derivation of a public key:
   $$\text{ActorID} = \text{SHA256}(\text{"NEX/ACTOR\_ID/v1"} \,\|\, \text{KeyType} \,\|\, \text{PublicKeyBytes})$$
2. **Web of Trust & Petnames:**
   Global naming authorities (DNS, ICANN, centralized handles) are rejected. Names are local petnames resolved transitively through the user's Web of Trust with exponential score dampening:
   $$\text{Score} = \text{Score}_A \times \text{Score}_B \times 0.5$$
