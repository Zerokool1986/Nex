# NEX-UX-FIGMA-WORKFLOW: Remote MCP Architecture, Visual Laboratory & Code-to-Canvas Integration

**Authority:** NEX Human Product Architecture  
**Status:** Authoritative Tooling & Workflow Specification  
**Classification Baseline:** `[Observed]`, `[Inferred]`, `[NEX-specific]`, `[Experimental]`  
**Date:** 2026-08-27  

---

## 1. The Role of Figma in the NEX Ecosystem

Figma is **not** an implementation shortcut or a source of code generation to paste blindly into `nex-desktop`. 

**The Visual Laboratory Principle:**
> *Figma is NEX's visual and interactive laboratory. It is the environment where we discover, model, and stress-test the human language of sovereign computing before locking that language into native Rust/egui/Android code.*

---

## 2. Remote Figma MCP Server Architecture

Figma provides an official hosted Remote MCP Server that allows AI agents to interact with Figma files via the standard Model Context Protocol:
- **Server Endpoint:** `https://mcp.figma.com/mcp`
- **Transport:** HTTP / Server-Sent Events (SSE)
- **Authentication:** OAuth 2.0 / Figma Personal Access Token

```text
  ┌─────────────────────────────────────────────────────────────┐
  │                   ANTIGRAVITY AGENT CLIENT                   │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ Model Context Protocol (MCP)
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │          OFFICIAL FIGMA REMOTE MCP SERVER (mcp.figma.com)    │
  └──────────────────────────────┬──────────────────────────────┘
                                 │ Figma REST & Plugin Engine
                                 ▼
  ┌─────────────────────────────────────────────────────────────┐
  │                     NEX FIGMA WORKSPACE                      │
  │  • Canvas Page 1: Design Tokens & Semantic Variables         │
  │  • Canvas Page 2: Atomic Components & Universal Inspector   │
  │  • Canvas Page 3: Visual Direction Matrix (A, B, C, D)      │
  │  • Canvas Page 4: Interactive Prototype (20-Step Journey)   │
  │  • Canvas Page 5: Experience Slider Stress Tests            │
  └─────────────────────────────────────────────────────────────┘
```

---

## 3. Remote MCP Tools & Capabilities Leveraged

The following official Figma MCP tools govern the visual workflow:

| MCP Tool Name | Execution Scope | Purpose in NEX Visual Laboratory |
|---|---|---|
| `create_new_file` | Remote Only | Initializes blank Figma Design and FigJam files for NEX explorations. |
| `use_figma` | Remote Only | Creates, modifies, and organizes frames, Auto Layout containers, components, variants, and variables. |
| `get_design_context` | Remote/Local | Extracts structured component styles, layout geometry, and token bindings from selected frames. |
| `get_variable_defs` | Remote/Local | Synchronizes design tokens (colors, radii, typography, spacing) between code and canvas. |
| `generate_diagram` | Remote Only | Converts NEX architectural Mermaid diagrams into editable FigJam visual workflows. |
| `get_screenshot` | Remote/Local | Renders visual snapshot of active frames for automated design review. |
| `download_assets` | Remote Only | Exports vector icons, badges, and image assets directly into the repository. |

---

## 4. Figma Canvas Architecture & Page Structure

To maintain absolute clarity, the master NEX Figma file is organized into 5 dedicated pages:

```text
NEX Design System & Visual Laboratory (Figma)
├── 📄 Page 1: Foundations & Design Tokens
│   ├── Color Variable Collections (Base, Surfaces, Accents, Trust, Status)
│   ├── Spacing & Radius Variables (4px Grid, 4px-16px Radii)
│   └── Typography Styles (Display, Headings, Body, Monospace Code)
│
├── 📄 Page 2: Component Library & Universal Inspector
│   ├── Atoms: Buttons, Inputs, Avatars, Trust Badges, Status Pills
│   ├── Molecules: Object Cards, Person Cards, Device Tiles, Sync Gauges
│   └── Organisms: Universal Object Inspector (Expanded & Collapsed States)
│
├── 📄 Page 3: Visual Directions Exploration (A / B / C / D)
│   ├── Frame A: Calm Sovereignty (Warm obsidian, tactile pills, editorial tone)
│   ├── Frame B: Modern Native (Translucent acrylic, platform sans, crisp borders)
│   ├── Frame C: Spatial Object-Centric (Canvas grid, floating nodes, connection strands)
│   └── Frame D: Minimalist Utility (Pure monochrome, dense text, hairline borders)
│
├── 📄 Page 4: Canonical Human Journey Interactive Prototype
│   ├── 01. Launch & Home Arrival (Family Space Filtered)
│   ├── 02. Photo Ingestion (Drag-and-Drop to Photos Lens)
│   ├── 03. Photos Viewport (Thumbnail Grid with 🟢 Local/Sync Badges)
│   ├── 04. Open Universal Object Inspector (Provenance, Safe on 3, Amy's Access)
│   ├── 05. Open Amy's Person Surface (Verified Trust, Shared Albums, Comms)
│   └── 06. Open Device Surface (Mesh Topology, Direct LAN Speed, Battery/Quota)
│
└── 📄 Page 5: 4-Tier Experience Slider Visual Matrix
    ├── Viewport @ Simple Tier (Zero jargon, automatic sync, calm status)
    ├── Viewport @ Standard Tier (Spaces switcher, quotas, capability roles)
    ├── Viewport @ Advanced Tier (Outbox queues, mesh transport, attenuated tokens)
    └── Viewport @ Expert / Operator Tier (SMT Merkle proofs, WAL frames, raw metrics)
```

---

## 5. Connecting Figma Design Back to Native Implementation

1. **Token Synchronization:** Design tokens declared in Figma Variables are exported via JSON and mapped to Rust structs in `nex-desktop/src/ui/palette.rs` and Android Compose `Theme.kt`.
2. **Code Connect Integration:** When components mature in the codebase, Code Connect mappings link the Rust/egui view functions to Figma component variants (`add_code_connect_map`).
3. **No Blind Copy-Paste:** Layouts prototyped in Figma serve as mathematical geometry benchmarks for Auto Layout to egui layout container translation.
