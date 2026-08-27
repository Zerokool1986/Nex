# Domain Docs

How the engineering skills should consume this repository's domain documentation when exploring the codebase.

## Single Source of Truth
The authoritative domain context, glossary, and architectural decisions for NEX live under the `NEX/` hierarchy and `.agents/`:

- **Master Context & Principles**: `NEX/01_MASTER_CONTEXT/NEX_MASTER_CONTEXT.md`
- **Domain Terminology & Glossary**: `NEX/01_MASTER_CONTEXT/NEX_TERMINOLOGY.md`
- **Architectural Mental Model**: `NEX/01_MASTER_CONTEXT/NEX_ARCHITECTURAL_MENTAL_MODEL.md`
- **Decision History & ADRs**: `NEX/01_MASTER_CONTEXT/NEX_DECISION_HISTORY.md`
- **Constitutional Invariants**: `NEX/00_CONSTITUTION/`
- **Subsystem Specs & Gates**: `NEX/02_SYSTEM/` and `NEX/04_GATES/INDEX.md`
- **Agent Rules & Authority Hierarchy**: `.agents/rules/`

## Before exploring, read these
Always consult `NEX/01_MASTER_CONTEXT/NEX_TERMINOLOGY.md` and `NEX/01_MASTER_CONTEXT/NEX_DECISION_HISTORY.md` before introducing new concepts or proposing structural changes.

## Use the Glossary's Vocabulary
When naming domain concepts, use the exact terms defined in `NEX/01_MASTER_CONTEXT/NEX_TERMINOLOGY.md`. Do not invent synonyms or deviate from established sovereign terminology.

## Respect Sealed Decisions and Invariants
All work is strictly governed by the NEX Authority Hierarchy (Level 1 Constitution -> Level 2 Frozen Contracts -> Level 3 Sealed ADRs -> Level 4 Sealed Gate Specs). Do not attempt to re-litigate or bypass frozen invariants.
