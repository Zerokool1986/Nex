# NEX-PRODUCT-ARCHITECTURE-TENSIONS: Open Product & Architecture Tensions Ledger (The Claude × Antigravity Dual-Agent Bridge)

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Open Tensions Ledger  
**Governance Hierarchy:** Level 1–2 Constitution & Frozen Wire/WAL → Level 3–4 ADRs & Sealed Gates → Level 4 NEX-UX-01 → UX Research Baseline → Human Product Baseline v1 → Open Tensions Ledger  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. Purpose of the Tensions Ledger

This ledger formalizes the active interface between **Claude's architectural scrutiny** (challenging invariants, threat models, economic viability, and transport boundaries) and **Antigravity's human product realization** (testing what humans can actually understand and use).

Instead of silently inventing answers or making unilateral assumptions in code, every unresolved architectural tension is explicitly recorded here with its **Human Product Desirability**, **Substrate Reality**, and **Open Architectural Question for Claude**.

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        THE DUAL-AGENT COLLABORATIVE STRUCTURE                          │
├────────────────────────────────────────┬───────────────────────────────────────────────┤
│ ANTIGRAVITY (Product Realization)      │ CLAUDE (Architectural Defense)                │
├────────────────────────────────────────┼───────────────────────────────────────────────┤
│ • "What do humans need to see & feel?" │ • "What can the mathematics actually guarantee?"│
│ • "How to make sovereignty intuitive?" │ • "Where does the threat model break down?"   │
│ • "How to eliminate technical dread?"  │ • "What are the economic & transport costs?"  │
└────────────────────────────────────────┴───────────────────────────────────────────────┘
```

---

## 2. The 6 Open Product / Architecture Tensions

```text
┌─────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                    OPEN TENSIONS LEDGER (CLAUDE × ANTIGRAVITY)                                  │
├─────┬──────────────────────────┬─────────────────────────────┬────────────────────────────┬─────────────────────┤
│ #   │ Subsystem Area           │ Human Product Desire (AG)   │ Substrate Reality / Cost   │ Open Question (CC)  │
├─────┼──────────────────────────┼─────────────────────────────┼────────────────────────────┼─────────────────────┤
│ T1  │ **Economics & Storage    │ Users want "Unlimited Sync" │ Replicas cost disk and     │ How to meter mesh   │
│     │ Incentives**             │ between all family devices  │ bandwidth; collaborative   │ storage without a   │
│     │                          │ without thinking of quotas. │ nodes require accounting.  │ speculative token?  │
├─────┼──────────────────────────┼─────────────────────────────┼────────────────────────────┼─────────────────────┤
│ T2  │ **Reticulum Mesh vs.     │ Users want fast sync over   │ Reticulum packet radio is  │ Is Reticulum strictly│
│     │ High-Bandwidth Wi-Fi**   │ any link; expect instant    │ low-bandwidth (bps/kbps);  │ discovery/metadata, │
│     │                          │ 50MB video replication.     │ bulk CAS requires TCP/QUIC.│ or bulk transport?  │
├─────┼──────────────────────────┼─────────────────────────────┼────────────────────────────┼─────────────────────┤
│ T3  │ **Synthetic & Composite  │ Users want virtual albums,  │ DAG nodes are immutable CAS│ Are albums first-   │
│     │ Objects in DAG**         │ smart search tags, and live │ blobs; mutable pointers    │ class objects or    │
│     │                          │ collections to sync fluidly.│ require Lamport CRDT state.│ ephemeral queries?  │
├─────┼──────────────────────────┼─────────────────────────────┼────────────────────────────┼─────────────────────┤
│ T4  │ **Provider Dependence &  │ Users want remote sync when │ Sovereign nodes behind NAT │ How to run relays   │
│     │ NAT Traversal**          │ outside home Wi-Fi without  │ require STUN/TURN/Relay    │ without centralizing│
│     │                          │ port forwarding routers.    │ infrastructure.            │ trust or custody?   │
├─────┼──────────────────────────┼─────────────────────────────┼────────────────────────────┼─────────────────────┤
│ T5  │ **Headless Node Pair &   │ Users want to pair a screen-│ SAS QR codes require a     │ What is the formal  │
│     │ Social Recovery**        │ less Raspberry Pi or NAS as │ camera and display on both │ headless pairing    │
│     │                          │ a home backup node easily.  │ devices for 4-word verify. │ protocol ceremony?  │
├─────┼──────────────────────────┼─────────────────────────────┼────────────────────────────┼─────────────────────┤
│ T6  │ **Production Hardening   │ Users expect seamless auto- │ Upgrades require schema    │ What is the frozen  │
│     │ vs. Proof-of-Concept**   │ updates and multi-platform  │ migrations across frozen   │ migration invariant │
│     │                          │ state compatibility.        │ WAL/WIRE framing formats.  │ for WAL v1 $\to$ v2?│
└─────┴──────────────────────────┴─────────────────────────────┴────────────────────────────┴─────────────────────┘
```

---

## 3. Deep Dive into Active Tensions

### Tension T1: Storage Economics vs. Family Mesh Sharing
- **Human Desirability (AG):** A parent creates a Family Space and expects all children's phones and household laptops to replicate family photos automatically. They do not want "gas fees," "token wallets," or micro-billing screens.
- **Architectural Constraint (Claude):** Unbounded replication across heterogenous devices exhausts small phone storage. Substrate credit accounting (`nex-core/src/apps/economics.rs`) tracks byte balances to prevent denial-of-service.
- **Resolution Path:** Experience Slider Stages:
  - *Simple / Standard:* Human quotas (*"Living Room Node has 2 TB available; Phone has 32 GB"*).
  - *Operator:* Substrate credit meters, CAS pruning policies, and erasure-coded slice distributions.

### Tension T2: Reticulum Role Boundary (Discovery vs. Bulk CAS)
- **Human Desirability (AG):** Users want seamless connectivity whether hiking in the mountains (mesh packet radio) or sitting in their living room (Gigabit Wi-Fi 7).
- **Architectural Constraint (Claude):** Reticulum over LoRa/packet radio provides ~1.2 kbps bandwidth. Transferring a 10 MB raw camera image over Reticulum would take 18 hours.
- **Resolution Path:** Strict Transport Decoupling:
  - Reticulum transports SMT frontiers, announcement beacons, text chat, and presence vectors.
  - High-speed TCP / QUIC / WebRTC transports bulk FastCDC content-addressed payload chunks.

---

## 4. Governance & Dual-Agent Bridge Protocol

1. **Antigravity Discipline:** Antigravity builds and tests human-facing projections of canonical state without mutating Level 1–4 contracts.
2. **Claude Scrutiny:** Claude evaluates this Tensions Ledger against the formal threat model, mathematical state invariants, and cryptographic proofs.
3. **Ratification:** When Claude and Antigravity align on a tension resolution, it is formally ratified as a Sealed ADR or Gate Specification before native implementation expands.
