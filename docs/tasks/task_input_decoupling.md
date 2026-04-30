# Task: Decouple Mouse and Keyboard Input Sources

Decoupled the input configuration so the pipeline can run two distinct input sources simultaneously (e.g., Hook for mouse and Raw Input for keyboard). This resolves the issue where a mouse-only backend (like `HookInputSource`) would deprive the system of keyboard events.

## Implementation Details

### Configuration Changes (`src/config.rs`)
- Replaced the single `input_method` field in `GeneralConfig` with two separate fields: `mouse_input_method` and `keyboard_input_method`.
- Updated `Config::default()` to default both methods to `"raw_input"`.

### Backend Enhancements (`src/input/raw_input.rs`)
- Updated `RawInputSource` to support selective device registration via `listen_mouse` and `listen_keyboard` flags.
- This prevents duplicate event processing when multiple instances of `RawInputSource` are active in the same pipeline (one for mouse, one for keyboard).
- Enhanced `RegisterClassW` handling to ignore `ERROR_CLASS_ALREADY_EXISTS`, allowing multiple instances to share the same message window class safely.

### Pipeline Orchestration (`src/pipeline.rs`)
- The `Pipeline` struct now manages two independent `Box<dyn InputSource>` instances: `mouse_input_source` and `keyboard_input_source`.
- `build_pipeline` instantiates these sources according to the separate config fields.
- `Pipeline::run` and `Pipeline::capture_one` now start both sources, passing a `.clone()` of the `mpsc::Sender<InputEvent>` to each. This ensures that all events (mouse and keyboard) are interleaved chronologically in the same trigger detector queue.

### User Interface Updates (`assets/index.html`)
- Split the "Input Method" dropdown in the Settings tab into two distinct selectors: "Mouse Input Method" and "Keyboard Input Method".
- Updated JavaScript synchronization logic (`populateSettings`, `saveConfig`) to handle the new dual-field configuration.

### Cross-Platform Compatibility
- Added `#[cfg(windows)]` gates to platform-specific code in `pipeline.rs`, `raw_input.rs`, `audio/mod.rs`, `ui/trace.rs`, and `output/keyboard.rs`.
- This ensures the project remains compilable and testable on non-Windows environments (like Linux CI or dev containers), even though the primary functionality is Win32-based.
