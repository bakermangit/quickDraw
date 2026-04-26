# Task: Gesture Trace Overlay

## Implementation Details

The gesture trace overlay is implemented as a lightweight Win32 popup window that provides real-time visual feedback when a gesture is being drawn.

### Architecture

- **Threading**: The overlay runs on its own dedicated OS thread to ensure it doesn't block the main gesture pipeline or the UI.
- **Window Styles**:
    - `WS_EX_TOPMOST`: Ensures the overlay is always visible above other windows.
    - `WS_EX_LAYERED`: Allows for per-pixel transparency.
    - `WS_EX_TRANSPARENT`: Makes the window "click-through", so it doesn't steal mouse focus or block interactions.
    - `WS_EX_TOOLWINDOW`: Prevents the window from appearing in the taskbar.
- **Rendering**: Uses native GDI for efficiency.
    - A 32-bit DIB (Device Independent Bitmap) section is used as a backbuffer.
    - `UpdateLayeredWindow` is called to sync the GDI DC to the window with alpha blending.
- **Communication**: The main pipeline sends `TraceCommand`s (Begin, AddPoint, End) via an MPSC channel. A custom `WM_USER_WAKE` message is used to wake up the message loop when a new command arrives.

### Configuration

- `general.trace_overlay_enabled` (bool): Enables or disables the overlay.
- `general.trace_color` (string): Hex color code for the trace line (e.g., "#00FF00").

### Win32 Caveats

- **Virtual Screen**: The overlay covers the entire virtual screen (`SM_XVIRTUALSCREEN`, `SM_YVIRTUALSCREEN`, etc.) to support multi-monitor setups. Coordinates must be offset by the virtual screen origin.
- **Transparency**: The DIB is cleared to 0 (fully transparent) at the start of each gesture. GDI drawing on the layered window DC must be synchronized using `UpdateLayeredWindow`.
- **Thread Safety**: `HWND` and other Win32 handles are not inherently `Send`. In Rust, they are wrapped to safely pass between threads where necessary, or kept within the thread that created them.
