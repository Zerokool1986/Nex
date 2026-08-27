# NEX-UX-DESIGN-SYSTEM: Design Token Architecture, Component Taxonomy & Semantic Variables

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Design System Specification  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. Design Token Architecture & Mathematical Base

The NEX Design System is structured as a 3-tier token hierarchy:
1. **Global Base Tokens:** Primitive color, font, spacing, and radius primitives.
2. **Semantic Tokens:** Contextual meanings (Surface, Text, Border, Status, Trust, Complexity).
3. **Component Tokens:** Surface-specific bindings for buttons, cards, inspector panels, and modals.

---

## 1.5. Brand Identity & The Interlocking N/X Geometric System

The official NEX brand identity is anchored by the **Interlocking N/X Geometric Symbol**:

```text
       ┌─────────── 1:1.618 ───────────┐
    ┌──┬───────────────────────────────┬──┐ ──▲──
    │  │      ██████       ██████      │  │   │
    │  │      ██   ██     ██   ██      │  │  A=B
    │  │      ██    ██   ██    ██      │  │   │
    │  │      ██     ██ ██     ██      │  │ ──▼──
    │  │      ██      ███      ██      │  │   ▲
  X:Y  │      ██     ██ ██     ██      │  │  X:Y
    │  │      ██    ██   ██    ██      │  │   │
    │  │      ██   ██     ██   ██      │  │   ▼
    │  │      ██████       ██████      │  │ ──▲──
    └──┴───────────────────────────────┴──┘  A=B
       └────────────── X:Y ────────────┘    ──▼──
```

### Geometric Construction & Proportions
- **Proportion Ratio:** Strict `1:1.618` Golden Ratio bounding box.
- **Corner Radii:** Inner/outer apex radius `r = 10px` normalized at 128px scale.
- **Topological Meaning:** An unbroken interlocking mesh chain expressing **Sovereign Connections**—the mathematical knot binding Identity, Devices, People, and Objects.

### Identity Lockup Configurations & Rules
1. **Horizontal Primary Lockup:** `[Interlocking Symbol]` + `NEX` (Geometric Sans wordmark with angled vertex cuts).
2. **Vertical / Badge Lockup:** `[Interlocking Symbol]` centered above `NEX`.
3. **Slogan Integration:** `[Interlocking Symbol]` + `NEX` + *"Sovereign connections."*
4. **Product Family Lockups:** `[Interlocking Symbol] Nex Drive`, `[Interlocking Symbol] Nex Photos`, `[Interlocking Symbol] Nex Media`, `[Interlocking Symbol] Nex Maps`.
5. **Contextual Surfaces Language:** The symbol serves as the central hub connecting the 4 primary relationship poles:
   - 🏠 **Home** (Top)
   - 👥 **People** (Left)
   - 📱 **Devices** (Right)
   - 👥👥 **Communities** (Bottom)

### Clear-Space & Scalability Rules
- **Clear-Space Boundary:** Minimum padding equal to `X` (the symbol height) on all 4 quadrants.
- **Raster Scaling Scale:**
  - `16px` (Favicon / Micro Status Badge) — Normalized stroke contrast.
  - `32px` (App Header / Breadcrumb) — Equal weight stroke clearance.
  - `64px` / `128px` (Contact & Space Avatar Badges).
  - `512px` / `1024px` (Squircle App Icon for macOS/Windows/Android).
- **Contrast Ratios:** Verified WCAG 2.1 AAA on Dark Obsidian (`#121216`) and High Contrast Light substrates.

```text
  ┌─────────────────────────────────────────────────────────────┐
  │ GLOBAL PRIMITIVES (e.g. `color.blue.500 = #5B8DF6`)          │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ (Aliased by Purpose)
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │ SEMANTIC TOKENS (e.g. `semantic.action.primary = {blue.500}`) │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ (Consumed by UI Elements)
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │ COMPONENT TOKENS (e.g. `button.primary.fill = {action.pri}`) │
  └─────────────────────────────────────────────────────────────┘
```

---

## 2. Spacing, Sizing & Layout Grid (4px Base Unit)

NEX uses a strict **4px/8px incremental spatial scale**:

| Token Name | Value (px) | Usage & Layout Intent |
|---|---|---|
| `space-1` | 4px | Micro padding inside compact badges, icon gap |
| `space-2` | 8px | Button internal padding, card item spacing, toolbar margin |
| `space-3` | 12px | Compact container padding, list item vertical gap |
| `space-4` | 16px | Standard card padding, sidebar navigation item padding |
| `space-5` | 20px | Canvas content margin, modal internal padding |
| `space-6` | 24px | Section header spacing, drawer interior margin |
| `space-8` | 32px | Major layout block separator, hero title margin |
| `space-12` | 48px | Empty state graphic container padding |

---

## 3. Typography Scale & Hierarchy

NEX combines a clear, highly legible humanist sans-serif for interface ergonomics with a refined serif accent for editorial personal titles.

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                             TYPOGRAPHY SCALE TABLE                               │
├───────────────────┬────────────┬─────────────┬────────────┬──────────────────────┤
│ Role              │ Size (px)  │ Weight      │ Line Ht    │ Primary Usage        │
├───────────────────┼────────────┼─────────────┼────────────┼──────────────────────┤
│ `display-lg`      │ 28px       │ SemiBold    │ 36px       │ Home welcome header  │
│ `heading-lg`      │ 22px       │ SemiBold    │ 28px       │ Lens main titles     │
│ `heading-md`      │ 18px       │ Medium      │ 24px       │ Section cards/drawers│
│ `body-lg`         │ 15px       │ Regular     │ 22px       │ Primary text/cards   │
│ `body-md`         │ 13.5px     │ Regular     │ 18px       │ Secondary descriptions│
│ `body-sm`         │ 12px       │ Regular     │ 16px       │ Status labels, badges│
│ `code-sm`         │ 11px       │ Monospace   │ 14px       │ ActorIDs, Merkle root│
└───────────────────┴────────────┴─────────────┴────────────┴──────────────────────┘
```

---

## 4. Semantic Color Tokens

### Surface & Elevation Tokens
- `surface.canvas`: `#121216` (Deep obsidian base canvas)
- `surface.sidebar`: `#18181E` (Dark graphite persistent sidebar & status bar)
- `surface.panel`: `#22222B` (Slate container cards, modals, inspector drawer)
- `surface.hover`: `#2A2A36` (Hovered row or card background)
- `surface.selected`: `#2E3A59` (Active selection highlight fill)

### Text & Icon Tokens
- `text.primary`: `#F0F0F5` (High contrast, readable white/cream)
- `text.secondary`: `#A0A0B2` (Muted captions, metadata labels)
- `text.tertiary`: `#6E6E82` (De-emphasized timestamps, inactive icons)
- `text.accent`: `#7AA2F7` (Interactive links, active tab labels)

### Status & Sovereignty Tokens
- `status.sync.verified`: `#34D399` (Emerald 🟢 — Verified SMT root, active LAN replication)
- `status.sync.replicating`: `#60A5FA` (Blue 🔵 — Active batch transfer in progress)
- `status.sync.local`: `#FBBF24` (Amber 🟡 — Object stored locally, awaiting remote peer)
- `status.sync.offline`: `#9CA3AF` (Gray ⚪ — Offline outbox queued)
- `status.danger`: `#F87171` (Red 🔴 — Key revocation, access blocked)

---

## 5. Shape & Corner Radii Scale

| Token Name | Value (px) | Usage |
|---|---|---|
| `radius-sm` | 4px | Checkboxes, tiny tag chips, tooltip bubbles |
| `radius-md` | 8px | Standard buttons, input fields, dropdown menus |
| `radius-lg` | 12px | Object cards, media thumbnails, notification toasts |
| `radius-xl` | 16px | Modals, inspector sliding drawer, large viewports |
| `radius-full` | 9999px | Avatars, status pills, Experience Slider thumb |

---

## 6. Motion & Micro-Interactions

NEX uses a physics-grounded, calm spring animation system:
- **Duration - Micro (Hover/Press):** `120ms` ease-out (`cubic-bezier(0, 0, 0.2, 1)`).
- **Duration - Panel Slide (Inspector Open):** `240ms` smooth deceleration (`cubic-bezier(0.16, 1, 0.3, 1)`).
- **Duration - Modal Overlay:** `180ms` ease-in-out.
- **Respects Reduced Motion `[Observed]`:** If OS reduced motion is active, all transitions instantly jump without animation.

---

## 7. Component Taxonomy (Atomic Inventory)

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        NEX COMPONENT TAXONOMY                          │
├───────────────────┬────────────────────────────────────────────────────┤
│ Tier              │ Components Included                                │
├───────────────────┼────────────────────────────────────────────────────┤
│ 1. Foundations    │ Color Swatches, Type Scale, Spacing Rules, Icons   │
│ 2. Atoms          │ Buttons, Inputs, Avatars, Status Pills, Badges     │
│ 3. Molecules      │ Object Card, Person Card, Device Tile, Search Bar  │
│ 4. Organisms      │ Universal Inspector, Top Bar, Lens Nav, Modals     │
│ 5. Templates      │ Home Viewport, Photos Grid, Drive Tree, Maps Canvas│
└───────────────────┴────────────────────────────────────────────────────┘
```
