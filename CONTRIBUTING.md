# Contributing to NEX

Thank you for your interest in contributing to NEX! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Constitutional Awareness](#constitutional-awareness)

## Code of Conduct

By participating in this project, you agree to maintain a respectful, inclusive, and collaborative environment. We are building technology that respects human sovereignty — our community should reflect the same values.

## Getting Started

### Prerequisites

- **Rust** (stable channel, 2021 edition) — install via [rustup.rs](https://rustup.rs/)
- **Platform build tools:**
  - Windows: Visual Studio Build Tools (C++ workload)
  - Linux: `build-essential`, `pkg-config`, `libgtk-3-dev`
  - macOS: Xcode Command Line Tools

### Clone & Build

```bash
git clone https://github.com/Zerokool1986/Nex.git
cd Nex
cargo build --workspace
cargo test --workspace
```

## How to Contribute

### Reporting Bugs

1. Search [existing issues](https://github.com/Zerokool1986/Nex/issues) to avoid duplicates
2. Use the **Bug Report** issue template
3. Include: environment details, reproduction steps, expected vs. actual behavior
4. If possible, include a minimal test case

### Suggesting Features

1. Open an issue using the **Feature Request** template
2. Describe the problem your feature solves
3. Explain how it relates to NEX's sovereignty model
4. Consider which [constitutional level](#constitutional-awareness) the feature affects

### Submitting Code

1. Fork the repository
2. Create a feature branch from `main` (`feat/your-feature-name`)
3. Write tests for your changes
4. Ensure all tests pass: `cargo test --workspace`
5. Format your code: `cargo fmt --all`
6. Submit a pull request using the PR template

## Development Setup

### Running the Desktop Application

```bash
cargo run -p nex-desktop
```

### Running Tests

```bash
# Run all workspace tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p nex-core
cargo test -p nex-desktop

# Run a specific test suite
cargo test -p nex-core r50_1
```

### Code Style

- Follow standard Rust conventions
- Use `cargo fmt --all` before committing
- Use `cargo clippy --workspace` to check for common issues
- Preserve existing comments and documentation that are unrelated to your changes

## Pull Request Process

1. **Fill out the PR template** completely
2. **Ensure all tests pass** — PRs with failing tests will not be merged
3. **One concern per PR** — keep changes focused and reviewable
4. **Authority level check** — identify which constitutional level your change affects
5. **No frozen contract violations** — changes to `NEX/WIRE/v1`, `NEX/WAL/v1`, C ABI v1, or sealed ADRs require explicit architectural review
6. **Update documentation** if your change affects public APIs or user-facing behavior

## Constitutional Awareness

> [!IMPORTANT]
> NEX is governed by an **8-level authority hierarchy**. All contributions must respect this structure — lower levels can never override higher levels.

Before submitting changes, determine which level your change affects:

| Level | What It Governs | Change Process |
|---|---|---|
| **1–2** | Constitution, Wire/WAL formats | **Frozen** — cannot be changed |
| **3–4** | ADRs, Gate specifications | Requires architectural review |
| **5** | FFI contracts, C ABI | Requires compatibility review |
| **6–7** | Rust implementation, tests | Standard PR review |
| **8** | Experimental work | Feature branch, standard review |

See [`NEX/00_CONSTITUTION/`](NEX/00_CONSTITUTION/) for the full constitutional specifications.

## Questions?

If you have questions about contributing, feel free to open a [discussion](https://github.com/Zerokool1986/Nex/discussions) or an issue.
