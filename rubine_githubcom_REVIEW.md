# Rubine PR — GitHub Copilot Review Findings

This document summarises every issue raised by the automated Copilot code-review
on PR **"Implement Simplified Rubine Recognizer"** (branch
`feat/rubine-recognizer-2113821462910593387`), together with a ready-to-paste
agent prompt that addresses all of them.

---

## Findings

### 1 · `src/main.rs:15` — Broken `use crate::types::SystemCommand` import

**Severity:** Compile error  
`main.rs` imports library crate modules through `use quickdraw::…` but still
uses `use crate::types::SystemCommand` on line 15.  Because `main.rs` is the
binary crate root, `crate` refers to the binary crate, not the library, so the
path does not resolve.

**Fix:** Change the import to `use quickdraw::types::SystemCommand;` (or simply
rely on the already-imported `types` alias, i.e. `types::SystemCommand`).

---

### 2 · `src/pipeline.rs:231-238` — `compute_speed()` uses wrong duration

**Severity:** Logic bug / inconsistency  
`compute_speed` divides total path length by
`timestamps.last().copied().unwrap_or(0)`.  
`GestureAccumulator` stores elapsed-since-start timestamps, but the **first**
timestamp may be non-zero (e.g. a small positive value captured at the moment
gesture recording began).  This under-estimates speed and makes UI-configured
speed filters inconsistent with the Rubine feature `f12` (which correctly uses
`t[n-1] - t[0]`).

**Fix:** Compute duration as `last - first` with a saturating subtraction, and
guard against the `len < 2` case:

```rust
fn compute_speed(capture: &GestureCapture) -> f64 {
    let length = compute_path_length(capture);
    let ts = &capture.timestamps;
    if ts.len() < 2 {
        return 0.0;
    }
    let duration = ts.last().unwrap().saturating_sub(*ts.first().unwrap());
    if duration == 0 {
        0.0
    } else {
        length / duration as f64
    }
}
```

---

### 3 · `src/pipeline.rs:262-265` — `"hook"` silently accepted for keyboard input

**Severity:** Silent feature gap  
`HookInputSource` only installs a `WH_MOUSE_LL` low-level hook; it never
captures keyboard events.  However, `build_pipeline` allows `"hook"` to be
configured as the `keyboard_input_method`.  When chosen, all keyboard-based
gesture triggers are silently dropped with no error or warning.

**Fix:** Return an explicit error for this combination:

```rust
let keyboard_input_source: Box<dyn InputSource> = match config.general.keyboard_input_method.as_str() {
    "raw_input" => Box::new(RawInputSource::new(false, true)),
    "hook" => return Err(anyhow!(
        "\"hook\" backend does not support keyboard input; use \"raw_input\" instead"
    )),
    other => return Err(anyhow!("Unknown keyboard input method: {}", other)),
};
```

---

### 4 · `src/gesture/rubine.rs:119-123` — Potential panic on non-monotonic or mismatched timestamps

**Severity:** Panic / crash  
`let dt = (t[i] - t[i - 1]) as f64;` performs **unchecked unsigned subtraction**
on `u64` timestamps.  If a capture loaded from a user-edited TOML file contains
non-monotonic timestamps, this will panic in debug builds (u64 overflow) and
silently wrap in release builds.  Additionally, if
`capture.timestamps.len() < capture.points.len()` the loop will index
out-of-bounds.

**Fix:** Add a length-mismatch guard at the top of `extract_features` and use
`saturating_sub` for the delta:

```rust
pub fn extract_features(capture: &GestureCapture) -> [f64; 13] {
    let n = capture.points.len();
    if n < 3 || capture.timestamps.len() < n {
        return [0.0; 13];
    }
    // ...
    let dt = t[i].saturating_sub(t[i - 1]) as f64;
    // ...
}
```

---

### 5 · `assets/index.html:802` — Dead/incorrect `volume` assignment per gesture

**Severity:** UI inconsistency / incorrect state  
`groups[g.name].volume = g.volume;` assumes a per-gesture `volume` field, but
the backend `GestureConfig` struct has no such field — volume is a global setting
under `[audio]`.  This assignment sets `undefined` on every group and can mislead
future UI developers.

**Fix:** Remove the line entirely, or replace it with the global config value
(`configData.audio.volume`) where a volume display is genuinely needed.

---

### 6 · `src/input/hook.rs:86-93` — `.expect()` panics the whole process on hook failure

**Severity:** Reliability  
`SetWindowsHookExW(...).expect("Failed to install mouse hook")` will crash the
entire daemon if the hook cannot be installed (insufficient permissions, OS
policy, accessibility restrictions, etc.).  Errors during hook installation
should be propagated back to `start()` so the engine can log and handle the
failure gracefully (e.g. fall back to `raw_input`).

**Fix:** Propagate the error instead of panicking:

```rust
let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), h_instance, 0)
    .map_err(|e| anyhow!("Failed to install mouse hook: {}", e))?;
```

Then return early from the thread, sending a notification through the existing
channel so `start()` can surface the error.

---

### 7 · `src/server/handlers.rs:152-163` — Mutex guard held across `.await`

**Severity:** Async correctness / potential deadlock  
In the `SetConfig` handler, `state_guard` (the `Mutex` lock guard) is held while
`state_guard.cmd_tx.send(SystemCommand::ReloadEngine).await` is called.  Holding
an `async` mutex guard across an `.await` point causes unnecessary contention and
can deadlock if any path triggered by the receiver also needs the same lock.

**Fix:** Clone `cmd_tx` while holding the lock, drop the guard, then await the
send:

```rust
ClientMessage::SetConfig { config } => {
    let cmd_tx = {
        let mut state_guard = state.lock().await;
        state_guard.config = config.clone();
        // … write to disk …
        state_guard.cmd_tx.clone()
    }; // guard dropped here
    let _ = cmd_tx.send(SystemCommand::ReloadEngine).await;
    let _ = tx.send(ServerMessage::Ok).await;
}
```

---

### 8 · `src/server/handlers.rs:242-245` — Same mutex-across-await in `ReloadEngine` handler

**Severity:** Async correctness / potential deadlock  
The `ReloadEngine` WebSocket handler also holds `state_guard` while awaiting
`cmd_tx.send(...)`:

```rust
ClientMessage::ReloadEngine => {
    let state_guard = state.lock().await;
    let _ = state_guard.cmd_tx.send(SystemCommand::ReloadEngine).await;
    let _ = tx.send(ServerMessage::Ok).await;
}
```

**Fix:** Same pattern — clone `cmd_tx`, drop the guard, then send:

```rust
ClientMessage::ReloadEngine => {
    let cmd_tx = state.lock().await.cmd_tx.clone();
    let _ = cmd_tx.send(SystemCommand::ReloadEngine).await;
    let _ = tx.send(ServerMessage::Ok).await;
}
```

---

### 9 · PR scope / description mismatch (documentation)

**Severity:** Process / maintainability  
The PR title and description focus exclusively on the Rubine recognizer, but the
change set also introduces hot-reload via `SystemCommand`, a trace overlay,
input-source decoupling, a Web UI overhaul, a repo-map generator tool, and speed/
length filters.  The mismatch makes code review, rollback scoping, and changelog
generation harder than necessary.

**Recommendation:** Update the PR description to list all feature areas, or split
the unrelated features into separate PRs.

---

## Agent Fix Prompt

Paste the following prompt into a fresh agent session (with access to this
repository) to address all actionable findings above:

---

```
You are working on the `feat/rubine-recognizer-2113821462910593387` branch of
the `bakermangit/quickDraw` repository.  Apply the following surgical fixes,
keeping changes as small as possible:

1. **src/main.rs line 15** — Change `use crate::types::SystemCommand;` to
   `use quickdraw::types::SystemCommand;` so the import resolves correctly
   within the binary crate.

2. **src/pipeline.rs `compute_speed()`** — Replace the current duration
   calculation (`timestamps.last().copied().unwrap_or(0)`) with
   `last.saturating_sub(first)`, guarding for fewer than 2 timestamps.
   This makes speed consistent with the Rubine f12 feature and UI filters.

3. **src/pipeline.rs `build_pipeline()` keyboard branch** — Change the `"hook"`
   arm for `keyboard_input_method` from constructing a `HookInputSource` to
   returning an explicit `Err(anyhow!(...))` with a message explaining that the
   hook backend only supports mouse input and `"raw_input"` should be used
   instead.

4. **src/gesture/rubine.rs `extract_features()`** — Add a guard at the top that
   returns `[0.0; 13]` if `capture.timestamps.len() < capture.points.len()`.
   Change `let dt = (t[i] - t[i - 1]) as f64;` to use `saturating_sub` so
   non-monotonic (user-edited) timestamps cannot cause a panic.

5. **assets/index.html line 802** — Remove the line
   `groups[g.name].volume = g.volume;`.  There is no per-gesture `volume`
   field on `GestureConfig`; volume is global under `[audio]`.

6. **src/input/hook.rs lines 86-93** — Replace the `.expect("Failed to install
   mouse hook")` call with a `map_err(|e| anyhow!(...))` that propagates the
   error back through `start()` instead of panicking the entire process.

7. **src/server/handlers.rs `SetConfig` handler (≈ line 152-163)** — Clone
   `cmd_tx` while still holding the mutex guard, then drop the guard before
   awaiting `cmd_tx.send(SystemCommand::ReloadEngine)`.  This prevents holding
   the async mutex across an `.await` point.

8. **src/server/handlers.rs `ReloadEngine` handler (≈ line 242-245)** — Apply
   the same fix: clone `state.lock().await.cmd_tx`, drop the guard immediately,
   then await the cloned sender.

After each change, run `cargo check` (and `cargo test` where relevant) to
confirm nothing is broken.  Do not alter any other files.
```
