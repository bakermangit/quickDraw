# Task: Web UI Desktop Overhaul and Daemon Restart

## Status: Completed

## Overview
This task involved a major overhaul of the QuickDraw web configuration interface to transition from a mobile-style vertical stack to a space-efficient, horizontal, desktop-first layout. Additionally, the backend was updated to support a seamless daemon restart command and more granular per-gesture configuration (audio overrides and explicit threshold overrides).

## Changes

### 1. Web UI (assets/index.html)
- **Desktop-First Layout**:
    - Converted `.form-group` elements to a horizontal layout using Flexbox.
    - Labels now sit on the left, with compact inputs/dropdowns (fixed 200px width) on the right.
- **Input Improvements**:
    - Replaced all wide `<input type="range">` sliders with compact `<input type="number">` boxes for precise control.
- **Persistent Settings Groups**:
    - Replaced the "Unhide" behavior with persistent visual boxes (`.persistent-box`).
    - When a feature (like Trace Finesse) is toggled off, the inputs within the box are now `disabled` (grayed out) rather than hidden.
- **Streamlined Modals**:
    - **Add Template**: Simplified to ONLY show the capture canvas. Gesture-level properties (name, action) are hidden as they are inherited from the parent gesture.
    - **Edit Gesture**:
        - Moved aggregate statistics (Min, Max, Avg for Length and Speed) to the top of the modal as a reference guide.
        - Added "Override Global Threshold" checkbox to explicitly enable/disable the per-gesture confidence threshold.
        - Exposed `sound` path and `volume` (0-100) inputs.
    - **Recording**: Removed auto-calculation buffers (±30%). Users now use the reference statistics to manually set their bounds.
- **Gesture List**:
    - Removed aggregate statistics from the main gesture header.
    - Added badges that dynamically show which filters (Speed/Length) are currently active for a gesture.
- **Gesture Templates Persistence**:
    - Implemented state tracking for the template preview lists. The "Templates" section for a gesture now remains open even after adding a new template or updating the list.
- **Recording Reset**:
    - Added a "Reset" button to the capture modal, allowing users to discard a recording and immediately try again.

### 2. Backend (src/server/handlers.rs & src/config.rs)
- **Restart Daemon Command (Experimental/Hidden)**:
    - Added `RestartDaemon` to the `ClientMessage` enum.
    - Implemented logic to spawn a fresh instance of the current executable and exit the current process using `std::env::current_exe()` and `std::process::Command`.
    - *Note: This feature is currently hidden from the UI as it experienced stability issues (crashing on subsequent restarts). The backend command remains for testing.*
- **New Gesture Config Fields**:
    - Added `volume: Option<f64>` to `GestureConfig` in `src/config.rs`.
    - Updated `UpdateGesture` message to support syncing `sound` and `volume` across all templates of a gesture.

## Architectural Notes for AI Architect

### Daemon Restart Mechanism
The backend now supports a `restart_daemon` WebSocket message. The implementation:
1. Resolves the current executable path using `std::env::current_exe()`.
2. Spawns a new process of that executable.
3. Terminates the current process immediately with `std::process::exit(0)`.
This allows the UI to trigger a full refresh of the application state (including input source re-registration) without manual user intervention.

### JSON Payload Updates
- **`UpdateGesture`**: Now includes `sound: Option<String>` and `volume: Option<f64>`.
- **`GestureConfig`**: Now includes a `volume` field (float, 0.0 to 1.0).

### Threshold Logic
The UI now distinguishes between "inherited" and "overridden" thresholds. If "Override Global Threshold" is unchecked, the UI sends `null` for `confidence_threshold`, signaling the pipeline to use the value from `GeneralConfig`.
