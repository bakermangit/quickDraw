# Raw Input Source Implementation Decisions

During the implementation of the `RawInputSource` in `src/input/raw_input.rs`, several technical ambiguities and compilation issues arose that required executive decisions, mostly stemming from the `windows` crate's strict typing and Rust's concurrency rules.

## 1. Thread Safety of `HWND` (`*mut c_void`)
**Issue:** The `windows-rs` crate defines `HWND` as a wrapper around `*mut core::ffi::c_void`, which means it does not implement the `Send` or `Sync` traits. However, we need to pass the created window handle from the background message loop thread back to the main `RawInputSource` struct so that the `stop()` method can send a quit message to it later.
**Decision:** I created a private newtype wrapper `struct SendHwnd(HWND)` and explicitly implemented `unsafe impl Send for SendHwnd {}` and `unsafe impl Sync for SendHwnd {}`. This is considered safe in this context because the `HWND` is strictly used across threads to post a window message (`PostMessageW`) to signal termination, rather than mutating state directly.

## 2. Window Procedure Signature (`WNDPROC`)
**Issue:** `WNDCLASSW` expects an `lpfnWndProc` of type `Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>`. Passing the standard `DefWindowProcW` directly resulted in a type mismatch because it is seen as a Rust fn item rather than an `extern "system"` fn pointer.
**Decision:** I created a trampoline function:
```rust
unsafe extern "system" fn raw_input_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}
```
This safely bridges the type mismatch and provides a valid window procedure for the hidden message-only window.

## 3. Graceful Thread Termination
**Issue:** `GetMessageW` is blocking. If `stop()` is called, simply setting a boolean flag won't wake the thread up from `GetMessageW` if no mouse events are occurring.
**Decision:** I utilized an `Arc<AtomicBool>` (`is_running`) coupled with an explicit window message. When `stop()` is invoked, we set `is_running` to false and execute `PostMessageW(hwnd, WM_USER + 1, WPARAM(0), LPARAM(0))`. The message loop checks for `WM_USER + 1` and breaks, allowing the thread to join cleanly.

## 4. Cleaning Up Raw Input Registration
**Issue:** The documentation specified stopping cleanly but didn't explicitly detail the Win32 tear-down.
**Decision:** On termination of the message loop, I explicitly use `RIDEV_REMOVE` with `RegisterRawInputDevices` passing a null `HWND` to unregister from system raw input before destroying the message window with `DestroyWindow`.

## 5. Timestamp Generation
**Issue:** `InputEvent` requires a monotonic `u64` timestamp in milliseconds, but Windows raw input doesn't provide a reliable absolute timestamp in the struct.
**Decision:** I utilized `std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)` to generate a cross-platform Unix timestamp in milliseconds for each event.

---

## Addendum — Architect Review (2026-04-23)

All 5 decisions accepted with no changes to source files.

### Confirmed: SendHwnd newtype wrapper
Classic and correct safe pattern for this exact Win32 scenario. `PostMessageW` is safe to call cross-thread on a valid `HWND` — the unsafe impl is justified.

### Confirmed: Trampoline wnd_proc
The type mismatch between Rust fn items and `extern "system"` fn pointers is a well-known `windows-rs` friction point. Trampoline is the standard fix.

### Confirmed: WM_USER+1 for thread wakeup
Correct approach. Setting an atomic flag alone cannot unblock a thread sitting in `GetMessageW`. The posted message is necessary.

### Confirmed: RIDEV_REMOVE on teardown
Good hygiene. Prevents the OS from trying to deliver messages to a destroyed window.

### Minor note: SystemTime vs GetTickCount64
`SystemTime::now()` works correctly but is not strictly monotonic — it can jump if the system clock is adjusted. `GetTickCount64()` from the `windows` crate is the proper monotonic Windows clock for event timestamps. Not worth changing now since the timestamp is only used for velocity/duration filtering (future feature), but log it for a future cleanup pass.

