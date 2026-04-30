# Task: Build the Repo Map Generator

## Context & Objective
We are optimizing our agentic workflow by introducing a deterministic, static Repo Map. Instead of agents blindly navigating the codebase or reading every single file (which wastes tokens and risks hallucinations), we want a single `docs/REPO_MAP.md` file that acts as an API skeleton.

Your objective is to build a standalone Rust script that uses the `syn` crate to parse the main project's AST and extract all public definitions into a dense, LLM-friendly Markdown file.

## Requirements

### 1. Project Setup
- Create a new Rust binary package in the `tools/repo_map_generator/` directory. Do not add this to the root `Cargo.toml` as a workspace yet; it should be completely standalone so it can be run via `cargo run --manifest-path tools/repo_map_generator/Cargo.toml`.
- Add the necessary dependencies to its `Cargo.toml`: `syn` (with the `full` and `visit` features if needed), `quote`, `walkdir`, and `anyhow`.

### 2. AST Parsing Logic
The tool must walk through the `src/` directory (relative to the project root) and read every `.rs` file.
For each file:
- Parse the file content into an AST using `syn::parse_file`.
- Extract only **public (`pub`)** items: `struct`, `enum`, `trait`, `fn`, and `type` aliases.
- Extract any `///` docstrings attached to those public items.
- **CRITICAL**: Completely ignore any item annotated with `#[cfg(test)]` or `#[test]`.
- **CRITICAL**: Skip the `tests/` directory entirely if you encounter it during the walk.

### 3. Formatting & Output
- Format the extracted data into clean Markdown. Group the signatures by their relative file path (e.g., `### src/pipeline.rs`).
- **CRITICAL**: Wrap the entire output inside XML tags: `<repo_map>` at the very top and `</repo_map>` at the very bottom. This acts as a hard boundary for LLMs.
- The script should write the final output directly to `docs/REPO_MAP.md`, overwriting it if it exists.

### 4. Git Hook
- Create a simple shell script or instructions on how to add this as a `pre-commit` hook so it runs automatically.
- **CRITICAL**: Ensure the pre-commit instructions explicitly include running `git add docs/REPO_MAP.md` after the generator finishes, otherwise the updated map will be left out of the commit.

## Definition of Done
1. [x] The `tools/repo_map_generator` crate compiles successfully.
2. [x] Running the tool generates a perfectly formatted `docs/REPO_MAP.md`.
3. [x] The generated map contains XML boundary tags (`<repo_map>`).
4. [x] The generated map is completely devoid of test modules and non-public implementation details.
5. [x] You have critically evaluated your own code. Is it bloated? Is it following standard Rust conventions?
6. [x] **Self-Documentation:** Write any design decisions, ambiguities you faced, and manual steps required (like setting up the Git hook) in the "Implementation Notes" section below.

---
> **Agent Note:** Do not modify `ARCHITECTURE.md` or `DESIGN_OVERVIEW.md`. Your scope is strictly limited to the implementation. High-level architectural documentation will be updated separately.

## Implementation Notes & Agent Feedback

### Design Decisions
- Utilized `syn` with the `visit` feature to parse `.rs` files and cleanly extract public AST elements (`ItemStruct`, `ItemEnum`, `ItemTrait`, `ItemFn`, `ItemType`).
- Leveraged `quote::ToTokens` to convert AST nodes back to formatted string tokens. Function bodies are intentionally stripped, leaving only the public `pub` visibility modifier and signature.
- Docstrings (`///`) are extracted dynamically from `#[doc]` attributes and preserved above their respective items.
- Items annotated with `#[test]` or `#[cfg(test)]` are dynamically ignored.
- Directory traversal uses `walkdir`, explicitly skipping any `tests` folders or hidden paths to maintain output density.

### Pre-commit hook setup
To ensure `docs/REPO_MAP.md` is always up-to-date and stays in sync with code changes, set up a pre-commit hook:

1. Create or open the file `.git/hooks/pre-commit` in your repository.
2. Add the following script:
   ```bash
   #!/bin/sh
   echo "Running repo map generator..."
   cargo run -q --manifest-path tools/repo_map_generator/Cargo.toml

   # CRITICAL: Stage the updated map so it's included in the current commit
   git add docs/REPO_MAP.md
   ```
3. Make the hook executable:
   ```bash
   chmod +x .git/hooks/pre-commit
   ```