# Task: Implement Engine Hot-Reloading

## Agent Instructions
**Please read this entire document carefully.** It contains the architectural plan for implementing Hot-Reloading in QuickDraw. 
1. Execute the implementation steps exactly as outlined below.
2. If you encounter any technical ambiguities, deviate from the plan, or make notable design decisions, **you MUST document them in the "Developer Implementation Notes" section at the bottom of this file.**
3. Do not modify the original plan steps; only append your notes to the bottom section.

---

## Objective
Implement a "hot-reloading" mechanism that allows QuickDraw to seamlessly rebuild its internal pipeline (re-applying new configs, inputs, and audio settings) without shutting down the daemon process or losing the system tray icon.

## Context
Process-level restarts (`std::process::exit(0)`) cause ghost tray icons on Windows and are generally unsafe for this architecture. Instead, we can cleanly drop the old `pipeline_fut` in the main event loop and rebuild the pipeline, which natively unhooks input capture and recreates the trace overlays. 

## Implementation Steps

### 1. Types (`src/types.rs`)
- Define a new `SystemCommand` enum to be used globally (replacing `TrayCommand`):
  ```rust
  #[derive(Debug, Clone)]
  pub enum SystemCommand {
      Quit,
      OpenConfig,
      ReloadEngine,
  }
  ```

### 2. System Tray (`src/tray/mod.rs`)
- Update `start_tray` to accept `mpsc::Sender<SystemCommand>` instead of `mpsc::Sender<TrayCommand>`.
- Add a new "Reload Engine" menu item.
- When the menu item is clicked, send `SystemCommand::ReloadEngine`.

### 3. Server State (`src/server/mod.rs`)
- Update `ServerState` to explicitly hold `capture_tx: mpsc::Sender<CaptureRequest>` and `cmd_tx: mpsc::Sender<SystemCommand>`.
  ```rust
  pub struct ServerState {
      pub config: Config,
      pub gesture_profile: GestureProfile,
      pub capture_tx: mpsc::Sender<CaptureRequest>,
      pub cmd_tx: mpsc::Sender<SystemCommand>,
  }
  ```
- Adjust `server::start` signature and internals so `capture_tx` isn't passed as a separate axum `State` argument (it's inside `SharedState` now).

### 4. WebSocket Handlers (`src/server/handlers.rs`)
- Remove `capture_tx` from the `handle_socket` function arguments, since it's now in the `state`.
- Update the `StartCapture` handler to extract `capture_tx` from the shared state when a capture begins.
- Add `ReloadEngine` to the `ClientMessage` enum.
- Update the `SetConfig` and `ReloadEngine` handlers to send `SystemCommand::ReloadEngine` through the global `cmd_tx`.

### 5. Main Event Loop (`src/main.rs`)
- Switch the `cmd_rx` channel to use `SystemCommand`.
- Pass `capture_tx` and `cmd_tx` into the initial `ServerState`.
- When receiving `SystemCommand::ReloadEngine` in the main select loop:
  1. Drop the current `pipeline_fut` to safely shut down existing input hooks.
  2. Load the fresh `config` and `gesture_profile` from disk.
  3. Create a new `(capture_tx, capture_rx)` pair.
  4. Update the `ServerState` with the new config, profile, and the new `capture_tx`.
  5. Build a new pipeline using the new config and `capture_rx`, then reassign `pipeline_fut = Box::pin(pipeline.run())`.

### 6. Frontend GUI (`assets/index.html`)
- Add a "Reload Engine" button to the Settings tab next to "Save Settings".
- Update the `saveConfig()` function to automatically send `ReloadEngine` over the WebSocket when "Save Settings" is clicked, so the user's new settings apply instantly without restarting the app.

## Definition of Done
- Clicking "Save Settings" instantly applies changes (e.g. enabling trace overlay) without needing to close the app.
- Clicking "Reload Engine" from the system tray successfully logs "Reloading engine..." and recreates the pipeline without creating ghost icons.
- `cargo check` passes with no errors.

---

## Developer Implementation Notes
*(Agent: Please log any ambiguities, design decisions, or deviations from the plan below this line)*

- **Ambiguities / Deviations:**
  - [To be filled by agent]
- **Design Decisions:**
  - [To be filled by agent]
