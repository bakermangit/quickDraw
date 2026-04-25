# Pipeline Implementation Decisions

During the implementation of the core gesture engine pipeline in `src/pipeline.rs` and the orchestrator in `src/main.rs`, several technical decisions were made to handle concurrency and system constraints cleanly.

## 1. Bridging the Synchronous/Asynchronous Gap
**Issue:** The `Pipeline::run` function is an `async` loop built on Tokio, but the `RawInputSource` relies on a highly synchronous Win32 message loop (`GetMessageW`) running on a dedicated OS thread. 
**Decision:** I changed the `InputSource` trait to accept a `tokio::sync::mpsc::Sender`. Inside the synchronous `RawInputSource` thread, I utilized `tx.blocking_send(event)` to push events into the Tokio channel. This safely and efficiently bridges the thread-to-async boundary without dropping events (which `try_send` might do) or requiring a complex async runtime context inside the raw input thread. A channel buffer size of 256 was explicitly chosen to absorb bursts of high-polling-rate mouse movement without causing backpressure on the Win32 message queue.

## 2. Cursor Position Tracking
**Issue:** The specification required saving the cursor `origin` when the gesture activates to perform a "cursor reset" after the gesture completes, preventing unwanted camera/mouse drift in games.
**Decision:** I utilized `windows::Win32::UI::WindowsAndMessaging::GetCursorPos` to grab the absolute screen coordinates (as `POINT`) when the trigger `WaitingForSecond` state transitions to `GestureActive`. When the gesture is complete, if `cursor_reset` is enabled, the pipeline executes `SetCursorPos(origin_pos.0 as i32, origin_pos.1 as i32)`.

## 3. Gesture Point Accumulation
**Issue:** Mouse events from Raw Input provide *relative* deltas (`dx`, `dy`), but the `$1` recognizer requires a contiguous path of absolute coordinate points.
**Decision:** The pipeline maintains running accumulators (`current_x`, `current_y`) starting at `(0.0, 0.0)` for the duration of the gesture. Each `GesturePoint(dx, dy)` signal simply adds the delta to the accumulators and pushes the current running sum to the capture buffer. This successfully translates relative physical movement into a coherent geometric path suitable for template matching.

## 4. Timestamp Generation
**Issue:** `GestureCapture` expects `timestamps` representing the elapsed milliseconds since the start of the gesture. 
**Decision:** I initialize a `std::time::Instant::now()` when the `GestureStarted` signal is received. As each subsequent point arrives, `start_time.elapsed().as_millis() as u64` is recorded. This provides a highly accurate, monotonic timeline for the gesture which can be used by future velocity-based gesture filters.

## 5. M1 Abort Edge Case
**Issue:** If the user holds `M1`, the state machine transitions to `WaitingForSecond`. What happens if they just release `M1` without ever pressing `M2` (e.g., normal clicking/dragging)?
**Decision:** As written in the spec, if `M1` is released while waiting, the state resets to `Idle` and the release event is cleanly emitted as `Pass`. I added a specific unit test (`test_trigger_abort_first_button`) to guarantee that standard clicking behavior is perfectly preserved and does not accidentally leak state.

---

## Addendum — Architect Review (2026-04-24)

All 5 decisions accepted with no changes to source files. Implementation reviewed against source and is clean throughout.

### Confirmed: blocking_send for sync→async bridge
`tx.blocking_send()` is the correct and idiomatic solution for driving a Tokio channel from a synchronous OS thread. It applies backpressure rather than silently dropping events under load, which is critical for gesture accuracy. `try_send` would have been wrong here.

### Confirmed: GetCursorPos at WaitingForSecond→GestureActive transition
Origin is captured at exactly the right moment — when the gesture is confirmed active, not when M1 first goes down. This means the saved position is as close as possible to where the user intends the cursor to return.

### Confirmed: Point accumulation from (0,0)
Running (x,y) sums starting from (0,0) is correct. The $1 recognizer's `translate_to_origin` step normalizes all points to centroid anyway, so the absolute starting position is irrelevant to recognition. What matters is that the relative shape of the path is preserved, which this approach achieves perfectly.

### Confirmed: std::time::Instant for timestamps
`Instant` is monotonic by definition, which supersedes the `SystemTime` concern flagged in the task4 addendum. The task4 note about switching to `GetTickCount64()` is now resolved — `Instant` is the better choice and it's already here.

### Confirmed: M1 abort test
`test_trigger_abort_first_button` covers a genuine edge case (M1 press + M1 release with no M2) that the main state machine test does not exercise. Critical for ensuring normal clicking is never disrupted.

### Code note: GestureStarted origin capture
On `GestureStarted`, the pipeline reads `origin_pos` by matching on `self.trigger.state` after the `process()` call. This works correctly because `process()` already transitioned the state to `GestureActive { origin }` before returning `GestureStarted`. Clean sequencing.