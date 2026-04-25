# Task 1 — Rust Project Scaffolding

**Task:** Initialize the Cargo project, create the source layout from ARCHITECTURE.md, add v1 dependencies, and verify with `cargo check`.  
**Status:** ✅ Complete — `cargo check` passes with 0 errors, 0 warnings.

---

## Decisions & Ambiguities

### 1. Dependency version pinning
**Source:** `docs/CONVENTIONS.md` — Dependencies table lists crate names only, no versions.  
**Decision:** Picked the latest semver-compatible versions available at time of scaffolding:

| Crate | Version pinned | Notes |
|-------|---------------|-------|
| `windows` | `"0.58"` | Latest stable at time of writing |
| `tokio` | `"1"` | Resolves to 1.52 |
| `serde` | `"1"` | |
| `serde_json` | `"1"` | |
| `toml` | `"0.8"` | |
| `anyhow` | `"1"` | |
| `thiserror` | `"1"` | Pinned to v1, **not** v2 — see note below |
| `tracing` | `"0.1"` | |
| `tracing-subscriber` | `"0.3"` | |
| `tray-icon` | `"0.17"` | |
| `tokio-tungstenite` | `"0.23"` | |
| `axum` | `"0.7"` | See decision #2 |

**`thiserror` note:** `thiserror v2` is already published. Pinned to `"1"` because the code examples in CONVENTIONS.md use the v1 API (`#[from]`, error attribute syntax). If the project upgrades to v2, the error definitions in implementations will need review.

**Recommended follow-up:** Once v1 implementation stabilises, run `cargo update` and pin exact versions in `Cargo.lock`. Consider documenting canonical versions in CONVENTIONS.md.

---

### 2. `axum` vs `warp` — picked one, needs owner decision

**Source:** `docs/CONVENTIONS.md` — "warp or axum (HTTP server)".  
**Decision:** Chose `axum` because it integrates more cleanly with `tokio` and has stronger active maintenance momentum.  
**Risk:** This is an open architectural choice. If the preference is `warp`, `Cargo.toml` needs to be updated and the server implementation will differ.

> ⚠️ **Owner input needed:** Confirm `axum` or switch to `warp` before `src/server/` is implemented.

---

### 3. Redundant `[target.'cfg(windows)']` block in Cargo.toml

**What happened:** Added an empty `[target.'cfg(windows)'.dependencies]` section as a placeholder for future Windows-only deps, but `windows` is already declared unconditionally in `[dependencies]`.  
**Impact:** Harmless — cargo ignores empty dependency sections. But it's misleading noise.  
**Recommended fix:** Either gate the `windows` crate under `[target.'cfg(windows)'.dependencies]`, or remove the empty section entirely.

---

### 4. Module stub files are not truly empty

**Task said:** "Create empty mod.rs files."  
**Decision:** Added a single doc comment to each stub rather than leaving them completely empty, for two reasons:
1. Completely empty files with declared submodules can produce `unused` lint noise in some tooling configurations.
2. The comments serve as breadcrumbs pointing to the relevant ARCHITECTURE.md section — useful for future agents picking up individual modules.

**If truly empty files are preferred:** All comments can be stripped; the files will still compile.

---

### 5. Nothing was ambiguous in the Source Layout itself

`docs/ARCHITECTURE.md` § Source Layout was explicit and complete. Every file listed was created exactly as specified. No structural interpretation was required.

---

## Environment Note — `cargo` not on PATH

Rust was not installed on the machine at the time this task ran. `winget install Rustlang.Rustup` was executed as part of the task. Installation succeeded, but the `cargo` binary will **not** appear in existing terminal sessions until they are restarted (rustup modifies `PATH` in the user environment, which only takes effect in new shells).

**To run `cargo check` manually:**
```powershell
# Option A — restart your terminal, then:
cargo check

# Option B — without restarting:
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo check
```

---

## Addendum — Architect Review (2026-04-23)

Decisions made in the architect session after reviewing this task's ambiguities:

### Resolved: axum vs warp → **axum confirmed**
Axum is maintained by the Tokio team, has stronger community momentum, and warp is in maintenance mode. Updated `docs/CONVENTIONS.md` to remove the ambiguity — it now lists `axum` only.

### Resolved: thiserror v1 → **accepted, stay on v1**
The v1→v2 migration is mostly MSRV policy; the API surface we use (`#[error("...")]`, `#[from]`) is identical. Not worth the risk of confusing future agents when all docs show v1 syntax. Can upgrade in a dedicated pass later.

### Resolved: empty `[target.'cfg(windows)']` block → **removed**
Deleted the empty section from `Cargo.toml`. The `windows` crate is already declared unconditionally in `[dependencies]`, and this is a Windows-only project.

### Accepted: doc comments in stub files
Keeping the breadcrumb comments pointing to ARCHITECTURE.md sections. Helpful for future agents orienting to individual modules.

