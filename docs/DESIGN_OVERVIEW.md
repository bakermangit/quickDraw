# Design Overview

## What Is QuickDraw?

QuickDraw is a system-wide mouse gesture recognition engine for Windows. The user performs a mouse gesture (a specific pattern of mouse movement while holding a trigger), and QuickDraw translates that gesture into an action (a keypress, mouse click, or arbitrary code execution).

**Primary use-case**: Gaming — specifically strategy games like Age of Empires 2 DE, where gesture-based hotkeys can complement or replace traditional keybindings. QuickDraw works in exclusive fullscreen.

**Secondary use-case**: General desktop productivity — launching apps, window management, or any action that could be triggered by a gesture.

## Goals

1. **Modularity**: Input capture, gesture recognition, and output actions are independent, swappable modules. Each can be replaced without touching the others.
2. **Lightweight**: No visible overlay, no constant UI. Just a tray icon. The daemon should consume negligible CPU/memory when idle and minimal resources during gesture capture.
3. **Anti-cheat compatibility**: Different games have different anti-cheat systems that may block certain input methods. Multiple input backends exist so the user can choose one that works.
4. **Low latency**: Gesture recognition and action execution must feel instantaneous. No perceptible delay between completing a gesture and the action firing.
5. **Configurable**: Trigger keys, gesture-to-action mappings, algorithm choice, and input method are all user-configurable via TOML files.

## Non-Goals (for now)

- **Overlay rendering**: No on-screen gesture trail visualization. May be added later.
- **Cross-platform**: Windows only. Linux/macOS support is out of scope.
- **Complex UI**: The config frontend is a separate concern, accessed on-demand. The core daemon is headless.

## Key Design Decisions

### Language: Rust

- Direct access to Win32 APIs via `windows-rs` crate
- No runtime or GC — predictable latency
- Trait system provides natural module interfaces without OOP inheritance
- Strong type system catches errors at compile time, which helps with AI-assisted development
- Cargo features can toggle implementations at compile time

### Architecture: Daemon + Web Frontend

The application is split into two concerns:

- **Core daemon**: Runs in the background, captures input, recognizes gestures, executes actions. Has a system tray icon for start/stop/quit.
- **Web frontend**: A separate, on-demand config UI served by the daemon on `localhost`. Communicates via WebSocket. Opened from the tray icon's "Configure..." menu item. Closing the browser tab has zero impact on the daemon.

### Config Format: TOML

- Human-readable, supports comments (critical for annotating gesture profiles)
- Rust ecosystem standard (Cargo.toml)
- Excellent serde support
- Clean syntax for the moderate nesting depth our config requires

JSON is used for IPC messages between daemon and frontend.

### Input Method (v1): Raw Input

- Works in exclusive fullscreen (most compatible with games)
- `RIDEV_INPUTSINK` flag allows background reception without interfering with the game
- Multiple applications can listen to Raw Input simultaneously — no conflict with games that also use it
- Standard Windows API — less likely to trigger anti-cheat than hooks
- **Read-only**: Raw Input can observe mouse events but **cannot block or intercept** them. Mouse movement during a gesture will still reach the game. This is mitigated by the cursor reset feature (see below).
- Future modules: low-level hooks (can block input), polling (also read-only)

### Gesture Recognition (v1): $1 Recognizer

- Template matching approach — works with a single recorded sample per gesture
- Low false-positive rate (critical in gaming — accidental action triggers are unacceptable)
- Simple implementation (~100 lines of core logic)
- Rotation and scale invariant
- Future modules: Rubine (velocity-native), velocity filter (composable with $1)

### Output (v1): Keyboard Simulation

- Simulate keypresses via `SendInput` Win32 API
- Future modules: mouse click simulation, arbitrary code/script execution

### Trigger Mechanism

The gesture trigger is configurable. Default: **Mouse1 + Mouse2**.

Trigger behavior (M1+M2 example):
1. M1 pressed → passed through immediately (no delay, no interception)
2. M2 pressed while M1 held → enter gesture mode, start recording mouse movement
3. Mouse movement captured as gesture data
4. Either button released → exit gesture mode, process captured gesture

This approach introduces **zero input latency** for normal M1 usage. The tradeoff is that M2's initial press reaches the game, which is acceptable for strategy games. Users in latency-sensitive scenarios can configure a keyboard modifier instead.

**Important**: Whether mouse movement is blocked during a gesture depends on the input method:
- **Raw Input / Polling**: Read-only — mouse movement still reaches the game during gestures. Mitigated by cursor reset (see below).
- **Hooks** (future): Can intercept and block mouse movement from reaching the game.

### Cursor Reset

Since Raw Input and polling cannot block mouse movement, the cursor will drift during a gesture. To mitigate this, a configurable **cursor reset** feature teleports the mouse back to its position when the gesture trigger was activated. This fires after gesture processing completes.

Config options:
- `cursor_reset = true` — always reset (default for Raw Input / polling)
- `cursor_reset = false` — never reset (useful if blocking via hooks)

Implemented via `SetCursorPos` Win32 API. The original cursor position is recorded when the trigger activates.

### Audio Feedback

Optional audio cues for gesture results:
- **Global sounds**: `success.wav` plays on a successful gesture match, `error.wav` plays on a failed match
- **Per-gesture sounds**: Individual gestures can override the global success sound with a custom audio file
- Sounds are configurable and can be disabled entirely

This helps the user know immediately whether their gesture was recognized, especially important when there's no visual overlay.

Config example:
```toml
[audio]
enabled = true
success = "sounds/success.wav"    # global default
error = "sounds/error.wav"         # global default

[[gestures]]
name = "flick-right"
action = { type = "key_press", key = "F1" }
sound = "sounds/flick.wav"         # overrides global success sound
```

### Gesture Creation

Gestures are recorded through the frontend:
1. Frontend sends `StartCapture` to daemon
2. Daemon enters capture mode and collects raw mouse data while trigger is held
3. On trigger release, daemon processes the raw data through the active recognition algorithm
4. Daemon sends the processed result + raw data back to frontend
5. User names the gesture and assigns an action
6. Frontend sends `SaveGesture` to daemon, which writes to the TOML config

**Raw data is always preserved** alongside the processed representation. This allows re-processing existing gestures when switching algorithms or adding features like velocity filtering.

### Velocity Support

For v1, velocity is not part of gesture matching. However:
- Raw captures include timestamps, so velocity data is always available
- A simple velocity filter can be layered on top of $1: check total gesture duration against a threshold
- More sophisticated per-segment velocity profiles are a future enhancement
- The modular architecture supports composing recognizers with filters without modifying either

## Programming Paradigm

Rust, not OOP. Specifically:

- **Plain structs with pub fields** for data types. No getters/setters.
- **Traits** for module interfaces. Describes capabilities, not identity. No inheritance.
- **Functional pipeline** for core data flow. Events flow through channels and transformations.
- **Message passing** (mpsc channels) for concurrency. No shared mutable state.
- **Static dispatch** (generics) preferred over dynamic dispatch (`dyn Trait`), except where runtime polymorphism is genuinely needed (e.g., user selects algorithm in config).
- **Declarative config** via serde derive macros.

What we explicitly avoid:
- Inheritance hierarchies
- God objects
- Design patterns for their own sake
- Shared mutable state
- Encapsulation theater (private fields with trivial pub getters)
