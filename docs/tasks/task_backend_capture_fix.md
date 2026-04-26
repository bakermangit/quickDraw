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
