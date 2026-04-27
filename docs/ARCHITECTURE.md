# Architecture

## System Overview

QuickDraw is structured as a pipeline of three modular stages connected by channels:

```
┌──────────────┐
│ Input Sources │
│  (Mouse)     │──────┐
│  (Keyboard)  │      │ Channel      ┌─────────────────┐     Channel      ┌───────────────┐
└──────────────┘      └─────────────►│  Gesture Engine  │────────────────►│ Output Action  │
                      InputEvent     │                 │   ActionRequest  │               │
                                     │ ($1 Recognizer) │                  │ (Keyboard Sim)│
                                     │ (Rubine)        │                  │ (Mouse Click) │
                                     │ (+ Filters)     │                  │ (Code Exec)   │
                                     └─────────────────┘                  └───────────────┘
        ▲                                                                      │
        │                        ┌─────────────────┐                           │
        │                        │   Config Store   │                           │
        │                        │   (TOML files)   │◄──────────────────────────┘
        │                        └────────┬─────────┘
        │                                 │
        │                        ┌────────▼─────────┐
        │                        │   Tray Icon       │
        │                        │   (start/stop/    │
        └────────────────────────│    configure/quit)│
                                 └────────┬─────────┘
                                          │
                                 ┌────────▼─────────┐
                                 │  WebSocket Server │◄────► Browser (Config UI)
                                 └──────────────────┘
```

## Core Pipeline

### Data Flow

```
1. Decoupled InputSources (Mouse & Keyboard) concurrently produce InputEvents (mouse move, button press/release).
2. Both feed a single MPSC channel. TriggerDetector consumes InputEvents, manages trigger state.
   - When trigger activates: begins accumulating mouse positions into a GestureCapture
   - When trigger deactivates: sends completed GestureCapture to GestureEngine
3. GestureEngine receives GestureCapture, runs recognition
   - If matched: produces ActionRequest (which gesture, confidence score)
   - If not matched: discards
4. OutputAction receives ActionRequest, executes the bound action
```

### Key Types

```rust
/// A raw mouse input event from any InputSource
pub struct InputEvent {
    pub event_type: InputEventType,
    pub timestamp: u64,  // ms, monotonic
}

pub enum InputEventType {
    MouseMove { dx: i32, dy: i32 },          // relative movement
    MouseButton { button: MouseButton, pressed: bool },
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// Accumulated mouse data during an active gesture
pub struct GestureCapture {
    pub points: Vec<(f64, f64)>,      // accumulated (x, y) positions
    pub timestamps: Vec<u64>,          // ms since gesture start
}

/// Result of a successful gesture recognition
pub struct GestureMatch {
    pub gesture_id: String,            // matches config entry
    pub confidence: f64,               // 0.0 - 1.0
}

/// An action to execute, resolved from config
pub enum ActionRequest {
    KeyPress { key: VirtualKey, modifiers: Vec<VirtualKey> },
    // Future: MouseClick, CodeExec, etc.
}
```

## Module Interfaces (Traits)

### InputSource

```rust
pub trait InputSource: Send + 'static {
    /// Start capturing input. Sends events through the provided channel.
    /// This should spawn its own thread/task and return immediately.
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<()>;

    /// Stop capturing input and clean up resources.
    fn stop(&mut self) -> Result<()>;

    /// Whether this input source can block/intercept events from reaching other apps.
    /// Raw Input and polling are read-only (false). Hooks can intercept (true).
    fn can_block(&self) -> bool;

    /// Human-readable name for logging/config (e.g., "raw_input", "hook")
    fn name(&self) -> &str;
}
```

**v1 implementation**: `RawInputSource` — uses `RegisterRawInputDevices` with `RIDEV_INPUTSINK`. Read-only (`can_block() = false`).

**Future implementations**: `HookInputSource` (low-level mouse hook, `can_block() = true`), `PollingInputSource` (GetAsyncKeyState polling, `can_block() = false`).

### GestureRecognizer

```rust
pub trait GestureRecognizer: Send + 'static {
    /// Attempt to recognize a gesture from captured mouse data.
    /// Returns the best match above the confidence threshold, or None.
    fn recognize(
        &self,
        capture: &GestureCapture,
        templates: &[GestureTemplate],
    ) -> Option<GestureMatch>;

    /// Human-readable name (e.g., "dollar_one", "rubine")
    fn name(&self) -> &str;
}
```

**v1 implementation**: `DollarOneRecognizer` — the $1 unistroke recognizer.

**Future implementations**: `RubineRecognizer`, plus composable `GestureFilter` trait for velocity checks.

### GestureFilter (composable, optional)

```rust
pub trait GestureFilter: Send + 'static {
    /// Post-recognition filter. Returns true if the gesture should be accepted.
    fn accept(
        &self,
        capture: &GestureCapture,
        template: &GestureTemplate,
    ) -> bool;
}
```

**Future implementations**: `DurationFilter` (total gesture time), `VelocityProfileFilter` (per-segment speed matching).

### OutputAction

```rust
pub trait OutputAction: Send + 'static {
    /// Execute the action.
    fn execute(&self) -> Result<()>;

    /// Human-readable name (e.g., "key_press", "mouse_click")
    fn name(&self) -> &str;
}
```

**v1 implementation**: `KeyPressAction` — uses `SendInput` Win32 API.

**Future implementations**: `MouseClickAction`, `CodeExecAction`.

## Trigger System

The trigger detector sits between the input source and the gesture engine. It is not a trait — it's core pipeline logic, since all input methods produce the same `InputEvent` type.

### Trigger Configuration

```rust
pub enum TriggerConfig {
    /// Two keys/buttons pressed simultaneously (e.g., Mouse1+Mouse2)
    Combo { key1: String, key2: String },
    /// Single key/button held
    Single { key1: String },
}
```

Keys are string names, matched case-insensitively at runtime. This allows triggers to be freely configured in TOML without Rust enum variants. Accepted values:
- Mouse buttons: `Mouse1` (left), `Mouse2` (right), `Mouse3` (middle), `Mouse4`, `Mouse5`
- Keyboard keys: `A`–`Z`, `0`–`9`, `F1`–`F24`, `Shift`, `Ctrl`, `Alt`, `Tab`, `Esc`, `Space`, etc.
- Aliases accepted: `Left`/`Mouse1`, `Right`/`Mouse2`, `Middle`/`Mouse3`, etc. (case-insensitive)

### M1+M2 Trigger State Machine

```
State: Idle
  M1 pressed → pass through, State: WaitingForSecond(M1)
  M2 pressed → pass through, State: WaitingForSecond(M2)

State: WaitingForSecond(first_button)
  second button pressed → record cursor position, State: GestureActive
  first button released → pass through, State: Idle

State: GestureActive
  Mouse move → accumulate into GestureCapture
             → if input source can_block(): event is intercepted (not forwarded)
             → if input source cannot block: event reaches game normally
  Any trigger button released →
    1. Process GestureCapture through recognizer
    2. If matched: execute bound action, play success sound (if configured)
    3. If not matched: play error sound (if configured)
    4. If cursor_reset enabled: SetCursorPos to recorded position
    5. State: Idle
```

Key properties:
- **The first button press is always passed through with zero latency.**
- **Cursor position is recorded** when entering GestureActive, for optional reset.
- **Input blocking is opportunistic** — only possible with hook-based input sources.

## Configuration

### File Structure

```
%APPDATA%/QuickDraw/
├── config.toml            # Main configuration
└── gestures/
    └── default.toml       # Default gesture profile
```

### Config Schema

```toml
# config.toml

[general]
input_method = "raw_input"        # "raw_input" | "hook" | "polling"
recognizer = "dollar_one"         # "dollar_one" | "rubine"
confidence_threshold = 0.80       # Minimum match confidence (0.0 - 1.0)
gesture_profile = "default"       # Name of active gesture profile
cursor_reset = true               # Teleport cursor back after gesture (recommended for raw_input/polling)

[trigger]
type = "combo"                    # "combo" | "single"
key1 = "Mouse1"                   # First key/button (left mouse button)
key2 = "Mouse2"                   # Second key/button (right mouse button)

[audio]
enabled = true
success = "sounds/success.wav"    # Played on successful gesture match
error = "sounds/error.wav"         # Played on failed match

[logging]
level = "warn"                    # "error" | "warn" | "info" | "debug" | "trace"

[server]
port = 9876                       # WebSocket/HTTP server port
```

### Gesture Profile Schema

```toml
# gestures/default.toml

[[gestures]]
name = "flick-right"
action = { type = "key_press", key = "F1" }
sound = "sounds/flick.wav"            # Optional: overrides global success sound
confidence_threshold = 0.75           # Optional: overrides global threshold for this gesture

[gestures.pattern]
algorithm = "dollar_one"
# Processed template points (machine-generated — do not edit by hand)
template_points = [[0.0, 0.5], [0.25, 0.5], [0.5, 0.5], [0.75, 0.5], [1.0, 0.5]]

[gestures.raw]
# Original recording (preserved for re-processing with future algorithms)
points = [[0.0, 0.0], [52.0, 3.0], [105.0, 5.0], [158.0, 2.0], [210.0, 4.0]]
timestamps = [0, 16, 33, 50, 66]

[[gestures]]
name = "L-shape"
action = { type = "key_press", key = "G", modifiers = ["Ctrl"] }

[gestures.pattern]
algorithm = "dollar_one"
template_points = [[0.0, 0.0], [0.0, 0.25], [0.0, 0.5], [0.0, 0.75], [0.0, 1.0], [0.25, 1.0], [0.5, 1.0]]

[gestures.raw]
points = [[0.0, 0.0], [2.0, 50.0], [1.0, 100.0], [3.0, 150.0], [0.0, 200.0], [50.0, 201.0], [100.0, 200.0]]
timestamps = [0, 16, 33, 50, 66, 83, 100]
```

## IPC Protocol (WebSocket)

The daemon hosts a WebSocket server on `localhost:{port}`. Messages are JSON.

### Frontend → Daemon

```json
{ "type": "get_config" }

{ "type": "set_config", "config": { ... } }

{ "type": "start_capture" }

{ "type": "cancel_capture" }

{ "type": "save_gesture", "gesture": { "name": "...", "action": { ... } } }

{ "type": "delete_gesture", "name": "..." }

{ "type": "reload" }
```

### Daemon → Frontend

```json
{ "type": "config", "data": { ... } }

{ "type": "capture_result", "raw": { "points": [...], "timestamps": [...] }, "processed": { ... } }

{ "type": "capture_cancelled" }

{ "type": "error", "message": "..." }

{ "type": "ok" }
```

## Module Registry

Modules are registered at compile time (no dynamic loading for v1). The main function wires up the selected implementations based on config:

```rust
// capture_rx is provided by main.rs so the WebSocket server can trigger gesture capture
fn build_pipeline(
    config: Config,
    capture_rx: mpsc::Receiver<CaptureRequest>,
) -> Result<Pipeline> {
    let input_source: Box<dyn InputSource> = match config.general.input_method.as_str() {
        "raw_input" => Box::new(RawInputSource::new()),
        other => return Err(anyhow!("Unknown input method: {}", other)),
    };

    let recognizer: Box<dyn GestureRecognizer> = match config.general.recognizer.as_str() {
        "dollar_one" => Box::new(DollarOneRecognizer::new()),
        other => return Err(anyhow!("Unknown recognizer: {}", other)),
    };

    // loads gesture profile, builds template + action + gesture_configs maps
    Ok(Pipeline { input_source, recognizer, audio, trigger, capture_request_rx, /* ... */ })
}
```

This uses dynamic dispatch (`Box<dyn InputSource>`) because the implementation is chosen at runtime from config. This is one of the few places where dynamic dispatch is appropriate.

`Pipeline` also owns: `AudioPlayer`, `TriggerDetector`, `gesture_configs: HashMap<String, GestureConfig>`, `templates: Vec<GestureTemplate>`, `actions: HashMap<String, Box<dyn OutputAction>>`, and `capture_request_rx`.

## Source Layout

```
src/
├── main.rs                      # Entry point, config loading, pipeline wiring
├── config.rs                    # Config types, TOML deserialization
├── pipeline.rs                  # Core pipeline orchestration, trigger state machine
├── types.rs                     # Shared types (InputEvent, GestureCapture, etc.)
├── input/
│   ├── mod.rs                   # InputSource trait definition
│   └── raw_input.rs             # Raw Input implementation
├── gesture/
│   ├── mod.rs                   # GestureRecognizer + GestureFilter trait definitions
│   └── dollar_one.rs            # $1 recognizer implementation
├── output/
│   ├── mod.rs                   # OutputAction trait definition
│   └── keyboard.rs              # Keyboard simulation implementation
├── audio/
│   └── mod.rs                   # Audio feedback (play success/error sounds)
├── tray/
│   └── mod.rs                   # System tray icon and menu
└── server/
    ├── mod.rs                   # WebSocket + HTTP server
    └── handlers.rs              # IPC message handlers
```

## Concurrency Model

```
OS thread: Tray icon (Win32 message loop)
  └── blocking_send(TrayCommand) ──────────────────────────────────┐
                                                                    │
OS thread: Raw Input (Win32 GetMessageW loop)                       │
  └── blocking_send(InputEvent) ──────────────────────────────────┐ │
                                                                   │ │
Tokio runtime (main thread):                                       │ │
  ├── Pipeline task (async)  ◄────────────────────────────────────┘ │
  │    ├── tokio::select! on InputEvent channel + CaptureRequest     │
  │    ├── TriggerDetector state machine                             │
  │    ├── GestureRecognizer                                         │
  │    ├── OutputAction execution (SendInput)                        │
  │    └── AudioPlayer (PlaySoundW, async)                           │
  │                                                                  │
  ├── WebSocket server task (async, tokio::spawn)                    │
  │    ├── Serves assets/index.html on GET /                         │
  │    ├── WebSocket IPC on GET /ws                                  │
  │    └── Shared state: Arc<tokio::sync::Mutex<ServerState>>        │
  │         (config + gesture_profile — only place Mutex is used)    │
  │                                                                  │
  └── tokio::select! on pipeline.run() + cmd_rx ◄───────────────────┘
       (TrayCommand::Quit → exit, OpenConfig → spawn browser)
```

Inter-component communication is via `tokio::sync::mpsc` channels. The one exception is `ServerState` which uses `Arc<tokio::sync::Mutex<>>` — necessary because the WebSocket server must share config/profile data across concurrent WebSocket connections.
