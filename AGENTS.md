# NEX Workspace Agent Configuration

## Agent Rules
See `.agents/rules/` for the authoritative NEX Constitution, Authority Hierarchy, and Architectural Invariants.

## NEX Agent Operating Policy

1. **Constitutional Precedence**: The NEX Constitution and 8-Level Authority Hierarchy (`.agents/rules/`) always supersede generic agent skills, MCP guidance, framework conventions, Fowler code smells, and refactoring recommendations.
2. **Authority-Level Check**: Before changing architecture, implementation, protocol, persistence, identity, synchronization, security, or UX behavior, the agent MUST determine which NEX Authority Level (Levels 1–8) and sealed Gate govern the affected area. Lower levels NEVER override higher levels.
3. **Frozen Contract Immutability**: Generic skills may NEVER modify or weaken frozen contracts and sealed boundaries, including:
   - `NEX/WIRE/v1` (Binary wire framing and 48-byte headers)
   - `NEX/WAL/v1` (Append-only Write-Ahead Log format)
   - C ABI v1 signatures and DirectByteBuffer JNI memory contracts
   - Sealed Architectural Decision Records (ADRs)
   - Sealed Gate specifications (`R50` through `R72`, `P0-1` through `P0-7`)
4. **Evidence-Driven Completion**: Claims of completion require executable evidence appropriate to the affected layer. Passing a generic unit test or satisfying a generic skill's heuristic completion criteria is insufficient when an authoritative test matrix pass, integration harness, physical multi-device test, or UI runtime verification is required.
5. **Context7 Usage**: When current third-party library, crate, or API behavior matters (e.g., Rust crates, `egui`/`eframe`, Android NDK/JNI, Gradle, Playwright), use Context7 to retrieve current authoritative documentation rather than relying on static model memory.
6. **Runtime Verification**: Use Playwright, Chrome DevTools, Android CLI tooling, or equivalent runtime tools when the task requires empirical UI, DOM, or platform verification. Do not substitute static source inspection for empirical runtime evidence.
7. **GitHub Integration**: Use GitHub MCP and `gh` CLI for the repository's external issue, PR, and review evidence surfaces when applicable.
8. **Generic Skill Containment**: Generic engineering skills (`tdd`, `implement`, `code-review`, `to-spec`, `to-tickets`, `wayfinder`, `grill-with-docs`, `diagnosing-bugs`, `improve-codebase-architecture`) are subordinate engineering aids. They do not establish NEX architecture, specifications, or authority.
9. **No Speculative Expansion**: Do not introduce new architecture, products, protocols, transports, dependencies, or infrastructure merely because a generic skill recommends them. All NEX capabilities must trace directly to the applicable authoritative product, gate, or specification layer.
10. **Minimal Tooling Principle**: Prefer the smallest existing tool and skill set capable of completing the task. Do not install additional plugins, MCP servers, or skills unless a concrete, demonstrated capability gap requires it.

## Agent Skills Configuration

### Issue Tracker
GitHub Issues via GitHub MCP and `gh` CLI. See `docs/agents/issue-tracker.md`.

### Domain Docs
NEX Multi-Tiered Context System under `NEX/` and `.agents/`. See `docs/agents/domain.md`.
