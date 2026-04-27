# Task: Web UI Desktop Overhaul and UX Refinements

## Status: Completed

## Overview
This task involved a major overhaul of the QuickDraw web configuration interface to transition from a mobile-style vertical stack to a space-efficient, horizontal, desktop-first layout. Additionally, the backend was updated for robust global audio control, streamlined gesture management, and various UX enhancements.

## Changes

### 1. Web UI (assets/index.html)
- **Desktop-First Layout**:
    - Converted `.form-group` elements to a horizontal layout using Flexbox.
    - Labels now sit on the left, with compact inputs/dropdowns (fixed 200px width) on the right.
    - Fixed specific alignment issues for combo trigger button inputs.
- **Input Improvements**:
    - Replaced all wide `<input type="range">` sliders with compact `<input type="number">` boxes for precise control.
- **Persistent Settings Groups**:
    - Replaced the "Unhide" behavior with persistent visual boxes (`.persistent-box`).
    - When a feature (like Trace Finesse or Audio) is toggled off, the inputs within the box are now `disabled` (grayed out) rather than hidden.
- **Streamlined Modals**:
    - **Add Template**: Simplified to ONLY show the capture canvas. Gesture-level properties are hidden as they are inherited. Added a **"Reset" button** to quickly discard a recording and try again.
    - **Edit Gesture**:
        - Moved aggregate statistics to the top of the modal within their respective filter areas.
        - Refined stats format: `Templates = X; Min = Y, Max = Z, Avg = A`.
        - Added toggleable overrides for both **Confidence Threshold** and **Custom Sound**.
- **Gesture List**:
    - Removed aggregate statistics from the main gesture header and replaced them with active filter badges (e.g., `Speed > 1.2`).
    - Implemented **Template Expansion Persistence**: The "Templates" section for a gesture now remains open even after adding a new template or updating the list.
- **Restart Daemon (Removed)**:
    - An experimental restart feature was removed from the UI due to stability issues on subsequent restarts.

### 2. Backend (src/audio/mod.rs, src/server/handlers.rs & src/config.rs)
- **Global Audio Volume**:
    - Added `volume: f64` to `AudioConfig` in `src/config.rs`.
    - Removed per-gesture volume overrides for simplicity and predictability.
    - Updated `AudioPlayer` to apply this global volume setting to all MCI playback commands.
- **MCI Reliability**:
    - Updated MCI `open` command to explicitly use `type waveaudio` for `.wav` files, fixing a bug where WAV files ignored volume settings.
- **Message Cleanup**:
    - Removed `RestartDaemon` from `ClientMessage`.
    - Removed `volume` field from `UpdateGesture` message.

## Architectural Notes for AI Architect

### Audio Volume Control (MCI)
To support volume adjustment, the application now utilizes MCI aliases (`qdsound`) for all fire-and-forget audio playback. The `setaudio qdsound volume to <0-1000>` command is used to apply the global configuration setting before each `play` command.

For robust volume support, the application now attempts to open all audio files (including `.wav`) using `type mpegvideo` (DirectShow). If this fails, it falls back to a standard auto-type open. Multiple volume command variants (`setaudio volume` and `set audio volume`) are issued to maximize driver compatibility. Standard `PlaySoundW` was bypassed entirely to maintain consistent volume control.

### JSON Payload Updates
- **`AudioConfig`**: Now includes a `volume` field (float, 0.0 to 1.0).
- **`UpdateGesture`**: No longer includes `volume`.

### Override Logic (Threshold & Sound)
The UI now distinguishes between "inherited" and "overridden" properties for both Confidence Thresholds and Custom Sounds.
- If "Override Global Threshold" is unchecked, `confidence_threshold` is sent as `null`.
- If "Custom Sound" is unchecked, `sound` is sent as `null`.
The backend pipeline interprets `null` as a signal to use the global defaults defined in the `Config` struct.
