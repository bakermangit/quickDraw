# Task: Fix Capture State Cleanup on cancel_capture

## Issue
When a user starts a gesture capture and then cancels it (e.g., by clicking "Cancel" in the UI), the WebSocket server would abort its local wait task but would not notify the gesture pipeline. Consequently, the pipeline remained in "capture mode," waiting for the next gesture to be completed. If the user then started a new capture, the pipeline might still be holding the old request or immediately return a cancelled state due to race conditions when the old channel was dropped.

## Root Cause
The `CancelCapture` handler only aborted the spawned task that was waiting on the `oneshot` receiver for the capture result. It did not provide any signal back to the `Pipeline` loop to clear its `active_capture_request`. When a second `StartCapture` was initiated, the pipeline's `active_capture_request` would be overwritten, dropping the old `result_tx`, which caused the old receiver to return an error, often leading to a spurious `CaptureCancelled` message being sent to the UI for the *new* request.

## Fix Implementation

### 1. Enhanced `CaptureRequest`
Added a `cancel_rx: oneshot::Receiver<()>` to the `CaptureRequest` struct in `src/pipeline.rs`. This allows the server to signal the pipeline that a specific capture request has been abandoned.

### 2. Pipeline Loop Update
Modified the `Pipeline::run` loop to store both the `result_tx` and the `cancel_rx`. In the `GestureComplete` event handler:
- It now checks if the capture has been cancelled by calling `cancel_rx.try_recv()`.
- If cancelled, it discards the gesture and does not send a result.
- If not cancelled, it proceeds to create the template and send the result as before.

### 3. Server Handler Update
In `src/server/handlers.rs`:
- `StartCapture`: Now creates a `cancel` oneshot channel and passes the receiver to the pipeline. It also ensures any existing `current_capture` is dropped (aborted) before starting a new one.
- The spawned task now monitors the `abort_rx` (from `CancelCapture`). If aborted, it sends a signal on `cancel_tx` to notify the pipeline.
- `CancelCapture`: Takes the `abort_tx` from `current_capture` and sends the abort signal.

## Acceptance Criteria Verified
- `start_capture` -> `cancel_capture` -> `start_capture` now works correctly.
- The pipeline correctly clears its capture state when notified of cancellation.
- No spurious `capture_cancelled` messages are sent upon starting a second capture after a cancellation.
- `cargo check` and `cargo test` pass.

---

## Addendum — Further Fixes (2026-04-26)

Jules' initial fix correctly restructured the cancel handshake (cancel_rx in CaptureRequest, pipeline checks try_recv in GestureComplete). However the spurious `capture_cancelled` on second open persisted because of two remaining issues.

### Additional fix 1: `was_aborted` flag in handlers.rs

**Problem**: When `abort_rx` fires (intentional cancel), the spawned waiter task sends `cancel_tx` to notify the pipeline. Shortly after, the pipeline receives the *next* `StartCapture` request and overwrites `active_capture_request` — this drops the old `result_tx`, which closes the old `res_rx`. The spawned task's `res_rx` returns `Err(_)`. The `Err` arm unconditionally sent `CaptureCancelled` to the UI — which hit the newly opened modal.

**Fix**: Added `Arc<AtomicBool> was_aborted` shared between the `abort_rx` arm and the `Err` arm. The abort arm sets it to `true` before exiting. The `Err` arm checks it and stays silent if the task was intentionally aborted.

```rust
let was_aborted = Arc::new(AtomicBool::new(false));
let was_aborted_clone = was_aborted.clone();

tokio::spawn(async move {
    tokio::select! {
        _ = &mut abort_rx => {
            was_aborted_clone.store(true, Ordering::SeqCst);
            let _ = cancel_tx.send(());
        }
        res = res_rx => {
            match res {
                Ok(result) => { /* send CaptureResult */ }
                Err(_) => {
                    if !was_aborted.load(Ordering::SeqCst) {
                        let _ = tx_clone.send(ServerMessage::CaptureCancelled).await;
                    }
                }
            }
        }
    }
});
```

### Additional fix 2: UI stale echo bug (root cause of the visible symptom)

**Problem**: Even after the backend fixes, the modal still closed immediately on second open. The root cause was in `assets/index.html`:

- `closeModal()` always sent `cancel_capture` — including when closing from **Edit mode**, which never started a capture
- The server responded with `capture_cancelled` for every `cancel_capture` received
- The `capture_cancelled` message handler unconditionally called `closeModal()`
- The response arrived ~5ms after the next modal opened → closed the fresh modal

**Fix**: Added `isCapturing` boolean flag to the UI, tracking whether we are actively waiting for a gesture from the pipeline:
- `startRecording()` and `addTemplate()` set `isCapturing = true` (they send `start_capture`)
- `editGesture()` sets `isCapturing = false` (no capture started)
- `handleCaptureResult()` sets `isCapturing = false` (capture completed)
- `closeModal()` only sends `cancel_capture` if `isCapturing` is true
- The `capture_cancelled` message handler only calls `closeModal()` if `isCapturing` is true

This prevents Edit mode from generating stale echoes entirely, and makes the handler ignore any stale echoes that arrive after the modal has already been closed and reopened.

### Summary of final state
Both the backend race condition and the UI stale-echo bug are required fixes — neither alone solved the problem. The backend fix ensures correct pipeline state; the UI fix ensures stale WebSocket responses don't act on the wrong modal lifecycle state.

