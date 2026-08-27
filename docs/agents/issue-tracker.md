# Issue Tracker: GitHub

Issues and specs for this repository live as GitHub issues, managed via the GitHub MCP server and `gh` CLI.

## Repository Configuration
- **Tracker**: GitHub Issues
- **Tooling**: GitHub MCP / `gh` CLI
- **Remote**: Inferred from `git remote -v` (when remote is configured) or managed via GitHub MCP.
- **Local Fallback**: If operating in an offline environment without git remote, tickets may temporarily be tracked under `.scratch/<feature>/`.

## Conventions
- **Create an issue**: `gh issue create --title "..." --body "..."`
- **Read an issue**: `gh issue view <number> --comments`
- **List issues**: `gh issue list --state open`
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

## Pull Requests as a Triage Surface
- **PRs as a request surface: no.**

## Wayfinding Operations
- **Map Issue**: An issue labeled `wayfinder:map`
- **Child Tickets**: Linked sub-issues or task items referencing `Part of #<map>`
