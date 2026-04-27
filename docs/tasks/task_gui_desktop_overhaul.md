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
- **Global Audio Volume**:
    - Replaced per-gesture volume settings with a single "Audio Volume" slider in the main Settings tab.
    - Switched all audio playback (including WAV) to use the Windows MCI (Media Control Interface) to enable consistent volume control.

### 2. Backend (src/server/handlers.rs & src/config.rs)
- **Restart Daemon (Removed)**:
    - An experimental `RestartDaemon` command was briefly implemented but subsequently removed from both the UI and backend due to stability issues (crashes on repeated use).
- **Audio Configuration**:
    - Added `volume: f64` to `AudioConfig` in `src/config.rs`.
    - Updated `AudioPlayer` to apply this global volume setting to all `mciSendStringW` playback commands.

## Architectural Notes for AI Architect

### Audio Volume Control (MCI)
To support volume adjustment, the application now utilizes MCI aliases (`qdsound`) for all fire-and-forget audio playback. The `setaudio qdsound volume to <0-1000>` command is used to apply the global configuration setting before each `play` command.

For `.wav` files, the MCI `open` command explicitly uses `type waveaudio` to ensure the correct system driver is used, as the default MCI mapping sometimes fails to support volume commands for WAV formats. Standard `PlaySoundW` was bypassed entirely to maintain consistent volume control.

### JSON Payload Updates
- **`AudioConfig`**: Now includes a `volume` field (float, 0.0 to 1.0).
- **`UpdateGesture`**: No longer includes `volume`.

### Override Logic (Threshold & Sound)
The UI now distinguishes between "inherited" and "overridden" properties for both Confidence Thresholds and Custom Sounds.
- If "Override Global Threshold" is unchecked, `confidence_threshold` is sent as `null`.
- If "Custom Sound" is unchecked, `sound` is sent as `null`.
The backend pipeline interprets `null` as a signal to use the global defaults defined in the `Config` struct.
