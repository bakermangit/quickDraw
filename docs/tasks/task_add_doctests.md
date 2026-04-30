# Task: Add Core API Doctests

## Context & Objective
We have updated our coding conventions to adopt a "Hybrid Approach" for documentation. Currently, our core APIs lack "Happy Path" usage examples (doctests), which makes it harder for both humans and other agents to consume these modules downstream.
Your objective is to add strategic doctests to the core public traits and utility functions in the codebase.

## Requirements

1. **Review the Conventions:**
   - Read `docs/CONVENTIONS.md` (specifically the "Documentation & Doctests" section) to understand the Hybrid Approach. Do NOT add doctests to simple DTOs, config structs, or boilerplate.

2. **Add Doctests to Core Traits and IPC Boundaries:**
   - Use `docs/REPO_MAP.md` to identify the core interfaces of the application (e.g., `InputSource` in `src/input/mod.rs`, `GestureRecognizer` in `src/gesture/mod.rs`, `OutputAction` in `src/output/mod.rs`).
   - Specifically target the shared types in `src/types.rs` (like `GestureMatch` and `ActionRequest`) that are sent over WebSockets. Write doctests showing what their serialized JSON looks like.
   - Add concise, executable ````rust ` doctests inside their `///` comments demonstrating a mock implementation or typical usage.

3. **Verify Doctests:**
   - Run `cargo test` and ensure that all new doctests compile and pass.

## Definition of Done
1. [ ] Core architectural traits and utilities have "Happy Path" doctests.
2. [ ] Simple structs/boilerplate are left alone (no unnecessary context bloat).
3. [ ] `cargo test` passes successfully, executing the new doctests.
4. [ ] **Self-Documentation:** Write any design decisions, ambiguities you faced, and manual steps required in the "Implementation Notes" section below.

---
> **Agent Note:** Do not modify `ARCHITECTURE.md` or `DESIGN_OVERVIEW.md`. Your scope is strictly limited to the implementation and local component documentation. High-level architectural documentation will be updated separately.

## Implementation Notes & Agent Feedback
*(Agent: Please write your design decisions, ambiguities, and setup instructions here before submitting the PR)*
