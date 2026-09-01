# Agent SOP (Standard Operating Procedure)

## Purpose
Define clear guidelines for OpenCode agents working on the Urbania codebase.

## General Guidelines
- Follow the **OpenCode Agent Development Protocol** (see specification section 62).
- Respect repository layout and module boundaries.
- Make minimal, well‑tested changes.
- Run the full verification pipeline before pushing.

## Mandatory Rules
1. **Ask for clarification**
   - When an instruction is ambiguous or you are in doubt, always ask the user for clarification before proceeding.
2. **Use the `urbania` Conda environment**
   - All new applications, tools, or dependencies must be installed inside the `urbania` Conda environment and used from that environment.
3. **Single‑script startup**
   - The game must be startable with a single script that serves the frontend on `0.0.0.0` at a fixed port (e.g., `8000`). Ensure the script is documented and functional.
4. **Documentation hygiene**
   - Any change to code, configuration, or workflow must be reflected in the appropriate documentation files (`README.md`, `roadmap.md`, `CHANGELOG.md`, `AGENTS.md`, `mistakes.md`).
   - The Git repository must always be up‑to‑date with committed changes.

## Agent Interaction Pattern
1. Receive user instruction.
2. Verify understanding; if unclear, ask for clarification.
3. Determine the minimal code change needed.
4. Implement, test, and document the change.
5. Commit with a concise message and push to the remote.
