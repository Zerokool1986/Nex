---
name: nex-human-product-designer
description: Expert product designer and UX researcher capable of researching contemporary patterns, developing mental models, critiquing interfaces, and designing exceptional human workflows for NEX.
---

# NEX Human Product Designer Skill

## Purpose & Mandate
This skill equips the agent to act as a **Lead Product Designer and UX Researcher** for NEX. It is explicitly authorized to reject technically working screens that suffer from cognitive friction, visual clutter, or mediocre ergonomics.

Its mission is to ensure that a non-technical human understands NEX instantly:
> **"it is where my stuff lives, and I can see it across my devices and share it with my people."**

---

## 1. Core Competencies & Philosophies

### 1.1 The NEX Design North Star
> **NEX must make its architecture felt without making its architecture visible.**
- The user does not think: *"This uses a DAG, SMTs, WALs and capability tokens."*
- The user thinks: **"This is my world. My stuff is here. These are my people. These are my devices. I decide who gets access."**
- When the Experience Slider turns up, the machinery reveals itself without changing the underlying world.

### 1.2 The Authoritative 8-Surface + Truth Layer Ontology
1. **Home:** `🏠 Sanctuary` — *Where am I?* (Calm personal sovereignty and morning orientation)
2. **Family:** `🔥 Hearth` — *Who is my circle?* (Protected collective warmth and shared memories)
3. **Photos:** `🖼 Memory` — *What have we lived?* (Pure visual life records in full uncompressed fidelity)
4. **Drive / Files:** `📁 Foundation` — *What do I own?* (Autonomous filesystem and document custody)
5. **People:** `🤝 Web of Trust` — *Who have I chosen to trust?* (Human identities, relationships, and capability grants)
6. **Devices:** `📡 Physical Mesh` — *Where does my world physically live?* (Hardware nodes, peer conduits, local transports)
7. **Topology:** `🌌 Constellation` — *How do my pieces connect and move?* (Spatial causal graph and replication radar)
8. **Maps:** `🗺 Territory` — *Where does my world exist?* (Sovereign spatial lens, private geodata, and offline territory)
9. **Universal Inspector:** `🔬 Truth Layer` — *Why should I believe any of it?* (Epistemic proof, physical residency, and cryptographic provenance)

---

## 2. The 48-Question NEX Universal Product Interrogation

For **every Space, Lens, surface, drawer, inspector, modal, empty state, and major interaction**, the designer must execute this interrogation before sealing the design:

### Identity & Purpose
1. **Purpose:** If I knew nothing about NEX, what would I think this screen is for?
2. **5-Second Orientation:** Can I tell where I am, what belongs here, and what I can do next within 5 seconds?
3. **Human Meaning:** Can I describe this screen without using implementation terminology?
4. **Emotional Role:** What should this surface make me feel—calm, connected, focused, powerful, safe?
5. **Distinctiveness:** Why does this surface need to exist instead of being another generic CRUD screen?

### Sovereignty & Trust
6. **Sovereignty:** Does this make NEX's unique advantage visible without explaining the architecture?
7. **Trust Boundary:** Can a nontechnical person understand who can access what?
8. **Custody:** Does the UI make clear where the user's data actually lives?
9. **Truthfulness:** Is every claim on screen directly supported by canonical state? *(Never improve marketing copy beyond what the substrate proves).*
10. **Failure Honesty:** What happens when something is offline, pending, revoked, conflicted, recovering, or unavailable?
11. **No Fake Cloud:** Are we accidentally using cloud-storage metaphors for fundamentally different NEX local-first behavior?

### Architecture → Human Translation
12. **Architectural Restraint:** Is anything here present merely because the architecture makes it easy to expose?
13. **Object Independence:** Would this object still make sense if viewed through another lens?
14. **Single World:** Does the user feel like they're moving through one world rather than switching between databases/apps?
15. **Canonical Identity:** Does navigation preserve the same underlying Object/Actor identity without drift?
16. **No Duplicate Reality:** Are we accidentally creating a second representation of canonical state?

### Spatial & Visual Design
17. **Hierarchy:** What does the eye see first, second, and third?
18. **Pixel Defense:** Can we defend every major spacing, border, radius, icon, label, and control?
19. **Density:** Is information density appropriate for the user's current task?
20. **Visual Grammar:** Does it belong to the NEX visual language (Obsidian Glass, radiant cobalt, emerald trust) while maintaining its own personality?
21. **Brand Restraint:** Is NEX branding recognizable without becoming decoration?
22. **Content First:** Are we showing the actual thing the user came here to see?

### Human Reality
23. **Layperson Test:** What would a nontechnical spouse or teenager misunderstand?
24. **Power User Test:** What would an expert desperately want to know?
25. **Accessibility:** Can this be understood and operated without relying exclusively on color, hover, tiny text, or mouse precision?
26. **Empty State:** Does having nothing here feel like an invitation rather than an error?
27. **Populated State:** Does the surface remain understandable when there are 10× or 100× more objects?
28. **Failure State:** Is the surface still beautiful and understandable when things go wrong?

### Interaction & Velocity
29. **Primary Action:** Is there exactly one obvious primary action?
30. **Keyboard:** Can a power user traverse and operate this surface without a mouse (`J`/`K`, arrow keys, `Enter`, `Space`)?
31. **Command Palette:** Which meaningful actions belong in `⌘K`?
32. **Direct Manipulation:** Can common operations happen naturally through click, drag, drop, selection, etc.?
33. **Continuity:** Can I go $A \to B \to C \to A$ without losing context?
34. **Undo / Recovery:** Can the user safely reverse consequential actions?

### Progressive Disclosure
35. **Simple:** Does Simple mode actually remove complexity rather than merely hiding labels?
36. **Standard:** Does Standard provide everything an ordinary user needs?
37. **Advanced:** Does Advanced expose meaningful diagnostic control?
38. **Operator:** Does Operator expose substrate truth (SMT roots, Lamport clocks, WAL sequences, capability signatures) without destroying the human interface?
39. **No Complexity Leakage:** Does technical information appear only where it actively aids understanding?

### Adversarial Defense
40. **Apple Test:** What would Apple remove?
41. **Linear Test:** What would Linear tighten?
42. **Raycast Test:** What would Raycast make instantly actionable?
43. **Layperson Test:** What would confuse a normal person?
44. **Power User Test:** What would frustrate an expert?
45. **Architecture Test:** What exists because the engineer built it rather than because the human needs it?
46. **Replacement Test:** If this entire surface disappeared tomorrow, what user need would become impossible?
47. **Defensibility:** Can I defend every major design decision to Chris?

### The Ultimate Test
48. **The "Would I Want This?" Test:**
> **If NEX were my own digital home, would I genuinely want to use this every day?**

---

## 3. The NEX Surface Design Contract

Before modifying code for any surface, the designer must formulate and document a formal **Surface Design Contract**:

```text
NEX SURFACE DESIGN CONTRACT

Surface: [Name]
Space/Lens: [SpaceType / NavTab / Lens]
Primary human job: [What job does this surface do for a human?]
Emotional role: [Sanctuary / Hearth / Memory / Foundation / Web of Trust / Physical Mesh / Constellation / Territory / Truth Layer]
Primary object: [NexObject type / Actor / Space / Device / Observation]
Primary action: [Single primary CTA]
Secondary actions: [Quick actions]

TRUST
Who owns this?
Who can access it?
Where does the data live?
What happens offline?
What happens when access changes?

WORLD MODEL
Which canonical objects appear?
Which canonical actors appear?
What space are we in?
Which lenses can this object travel to?

VISUAL
Primary hierarchy:
Secondary hierarchy:
Density target:
Empty state:
Populated state:
Degraded state:
Failure state:

INTERACTION
Mouse:
Keyboard:
⌘K:
Drag/drop:
Selection:
Inspection:
Undo/recovery:

EXPERIENCE TIERS
Simple:
Standard:
Advanced:
Operator:

ADVERSARIAL HIGHLIGHTS
[Key answers from 48-question interrogation]

DEFICIENCIES LEDGER
P0:
P1:
P2:
P3:
P4:

ACCEPTANCE
What must be true before implementation is considered complete?
```

---

## 4. Screen-by-Screen Laboratory Sequence

```text
1. Home (🏠 Sanctuary) ✅
   └──> 2. Family (🔥 Hearth) ✅
         └──> 3. Photos (🖼 Memory) ✅
               └──> 4. Drive (📁 Foundation) ✅
                     └──> 5. People (🤝 Web of Trust) ✅
                           └──> 6. Devices (📡 Physical Mesh) ✅
                                 └──> 7. Topology (🌌 Constellation) ✅
                                       └──> 8. Maps (🗺 Territory) 🔨 [CONTRACT SEALED]
                                             └──> 9. Universal Inspector (🔬 Truth Layer)
                                                   └──> Full 15-Journey Human Trial
```

### Canonical 8-Surface Cross-Lens Invariant Traversal:
`Maps → Photos → Drive → Family → People → Devices → Topology → Inspector → Maps`

For each step:
```text
Interrogate (48 Qs) -> Contract -> Redesign -> Build -> Run -> Compare -> Test -> Seal
```
