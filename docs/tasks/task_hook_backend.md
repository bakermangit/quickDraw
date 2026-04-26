# Task: HookInputSource (WH_MOUSE_LL)

Implemented a low-level mouse hook (`WH_MOUSE_LL`) backend for the `InputSource` trait.

## Implementation Details

### Win32 Global State
Because the Win32 `LowLevelMouseProc` callback does not accept a user-data pointer, state must be managed via thread-safe global statics:

- `EVENT_TX`: A `OnceLock<mpsc::Sender<InputEvent>>` used to send captured events back to the pipeline.
- `SHOULD_BLOCK`: An `AtomicBool` that determines whether mouse events should be swallowed (returned as `1` in the hook) or passed through.
- `LAST_X` / `LAST_Y`: `AtomicI32` variables used to store the last absolute mouse position to calculate relative `dx` and `dy`.
- `HOOK_HANDLE`: A `OnceLock<HHOOK>` storing the handle returned by `SetWindowsHookExW`.

### Threading and Lifecycle
- `start()`: Spawns a new OS thread. Inside this thread, `SetWindowsHookExW` is called to install the hook. A standard `GetMessageW` loop is then run to process hook events. The thread's ID is captured to allow graceful shutdown.
- `stop()`: Uses `PostThreadMessageW` with `WM_QUIT` to break the message loop in the hook thread. The thread then calls `UnhookWindowsHookEx` before exiting.

### Coordinate Translation
Low-level mouse hooks provide absolute screen coordinates. `HookInputSource` tracks the previous position and calculates the relative difference (`dx`, `dy`) for each `WM_MOUSEMOVE` event, ensuring consistency with other input sources like `RawInputSource`.

### Event Mapping
The hook handles the following Win32 messages:
- `WM_MOUSEMOVE` -> `InputEventType::MouseMove`
- `WM_LBUTTONDOWN` / `WM_LBUTTONUP` -> `InputEventType::MouseButton { button: Left }`
- `WM_RBUTTONDOWN` / `WM_RBUTTONUP` -> `InputEventType::MouseButton { button: Right }`
- `WM_MBUTTONDOWN` / `WM_MBUTTONUP` -> `InputEventType::MouseButton { button: Middle }`
- `WM_XBUTTONDOWN` / `WM_XBUTTONUP` -> `InputEventType::MouseButton { button: X1/X2 }`

## Addendum: Keyboard Input Deprivation
Because `HookInputSource` exclusively implements `WH_MOUSE_LL`, the pipeline stops receiving `KeyboardKey` events entirely when this method is selected (since the global `RawInputSource` is disabled). A future architectural update will decouple mouse and keyboard input sources, allowing users to configure `mouse_input_method` and `keyboard_input_method` independently.
