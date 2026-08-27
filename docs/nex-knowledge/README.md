# NEX Repository Authoritative Knowledge Layer

**Status:** Evidence-Backed Architectural Baseline  
**Authority Hierarchy:** Level 1 (NEX Constitution) $\to$ Level 8 (Experimental)  
**Target Audience:** Autonomous Agents & Independent Claude Instances utilizing GitHub MCP / Filesystem Inspection  

---

## 1. Purpose of this Knowledge Base

This directory (`docs/nex-knowledge/`) establishes the authoritative, evidence-backed knowledge layer for the **NEX Sovereign Platform**. It enables independent reasoning and auditability directly from repository artifacts, source code, frozen contracts, and empirical test matrices, without reliance on transient conversational memory or unverified summaries.

### Epistemic Tagging Standards
Every claim within this knowledge base is strictly partitioned using the following epistemology:
- `[DIRECT SOURCE FACT]`: Verbatim citations from constitutional markdown files, frozen specifications, or sealed ADRs.
- `[IMPLEMENTATION OBSERVATION]`: Verified behaviors, structs, enums, functions, and layout observed in `nex-core`, `nex-desktop`, and `android` code.
- `[TEST EVIDENCE]`: Specific test assertions, suite names, and empirical boundaries observed in `nex-core/tests/`.
- `[INFERENCE]`: Architectural conclusions derived logically from combinations of facts, explicitly labeled.
- `[OPEN QUESTION]`: Unresolved seams, implementation divergences, or undocumented areas recorded without speculative resolution.
