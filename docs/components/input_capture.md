# Component: Input Capture

## Overview

The input capture module is responsible for reading raw mouse events from the operating system and publishing them as `InputEvent` values through a channel. Each implementation of the `InputSource` trait represents a different method of capturing mouse input.

The input source does NOT interpret triggers or gestures — it only produces a stream of mouse events. Trigger detection happens downstream in the pipeline.

## Interface

```rust
pub trait InputSource: Send + 'static {
    /// Start capturing input. Sends events through the provided channel.
    /// This should spawn its own thread/task and return immediately.
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<()>;

    /// Stop capturing input and clean up resources.
    fn stop(&mut self) -> Result<()>;

    /// Human-readable name for logging/config
    fn name(&self) -> &str;
}
```

### Contract

- `start()` must be non-blocking. Spawn a thread or task internally.
- Events must be sent through the provided `Sender<InputEvent>`.
- `stop()` must cleanly shut down the capture thread and release all OS resources.
- Multiple calls to `start()` without `stop()` should return an error, not create duplicate listeners.
- Mouse movement events should use **relative** deltas (`dx`, `dy`), not absolute coordinates.
- Button events must report which button and whether it was pressed or released.

## Implementation: Raw Input (v1)

### How It Works

1. Create a hidden message-only window (`HWND_MESSAGE`) to receive `WM_INPUT` messages
2. Call `RegisterRawInputDevices` with:
   - `usUsagePage = HID_USAGE_PAGE_GENERIC` (0x01)
   - `usUsage = HID_USAGE_GENERIC_MOUSE` (0x02)
   - `dwFlags = RIDEV_INPUTSINK` (receive input even when not in foreground)
   - `hwndTarget` = the hidden window handle
3. Run a message loop on a dedicated thread
4. On `WM_INPUT`, call `GetRawInputData` to extract `RAWINPUT` struct
5. Parse mouse data from `RAWINPUT.data.mouse`:
   - `lLastX`, `lLastY` → relative movement (when `MOUSE_MOVE_RELATIVE` flag is set)
   - `usButtonFlags` → button press/release events
6. Convert to `InputEvent` and send through channel

### Win32 API Calls

| API | Purpose |
|-----|---------|
| `CreateWindowExW` | Create hidden message-only window |
| `RegisterRawInputDevices` | Register for mouse raw input |
| `GetRawInputData` | Extract raw input data from `WM_INPUT` |
| `GetMessageW` / `PeekMessageW` | Message loop |
| `DestroyWindow` | Cleanup |

### Key Flags

- `RIDEV_INPUTSINK` — Receive input when window is not in foreground. Essential for background operation and exclusive fullscreen games.
- `MOUSE_MOVE_RELATIVE` — Check this flag in `RAWMOUSE.usFlags` to confirm movement data is relative (not absolute).

### Exclusive Fullscreen Compatibility

Raw Input works in exclusive fullscreen because:
- It operates at the HID level, below DirectInput and window message routing
- `RIDEV_INPUTSINK` explicitly enables background reception
- Multiple applications can register for raw input simultaneously without conflict
- Games that also use Raw Input are not affected — Windows delivers events to all registered listeners independently

### Anti-Cheat Considerations

Raw Input uses standard, documented Win32 APIs. It does not:
- Install hooks (no `SetWindowsHookEx`)
- Inject DLLs
- Modify memory of other processes
- Use undocumented APIs

This makes it the least likely input method to trigger anti-cheat. However, some aggressive anti-cheat systems may still flag it. This is why alternative input sources (hooks, polling) exist as modules.

## Future Implementations

### Hook Input Source

Uses `SetWindowsHookEx` with `WH_MOUSE_LL` to install a low-level mouse hook. More intrusive than Raw Input but may work in edge cases where Raw Input doesn't.

### Polling Input Source

Uses `GetCursorPos` and `GetAsyncKeyState` on a timer. Least intrusive (no hooks, no registration) but has higher latency (depends on poll interval) and misses rapid movements.

## Tasks

### v1: Raw Input Source

- [ ] Create `src/input/mod.rs` with `InputSource` trait definition and `InputEvent` type
- [ ] Create `src/input/raw_input.rs` with `RawInputSource` struct
- [ ] Implement hidden message-only window creation
- [ ] Implement `RegisterRawInputDevices` for mouse input with `RIDEV_INPUTSINK`
- [ ] Implement message loop on dedicated thread
- [ ] Parse `WM_INPUT` → `RAWINPUT` → `InputEvent` conversion
- [ ] Handle mouse move events (relative dx/dy)
- [ ] Handle mouse button events (all 5 buttons: left, right, middle, X1, X2)
- [ ] Implement `stop()`: signal thread to exit, destroy window, unregister devices
- [ ] Guard against double-start (return error if already running)
- [ ] Unit tests for event parsing logic (mock raw input data)
- [ ] Integration test: start source, move mouse, verify events received
