# Task 11: Web-based Configuration UI

## Objective
Implement a web-based configuration UI for QuickDraw that communicates with the daemon over an HTTP + WebSocket server.

## Decisions Made

1.  **Server Implementation:**
    *   Used `axum` with the `ws` feature to handle both HTTP routes and WebSocket upgrades gracefully.
    *   The state (`Config` and `GestureProfile`) is shared using `Arc<tokio::sync::Mutex<ServerState>>` injected via axum's state extractor.
    *   Created `src/server/mod.rs` to wire up the router and `src/server/handlers.rs` for WebSocket message processing.

2.  **WebSocket Protocol:**
    *   Adopted JSON serialization for IPC using `serde_json`. Messages implement `{ "type": "...", ... }` structure.
    *   Separated client and server messages into enum variants for structured handling.
    *   Operations like `save_gesture` and `set_config` update both the shared state and persist changes to the TOML files.

3.  **Pipeline Integration:**
    *   Introduced `CaptureRequest` and `CaptureResult` in `src/pipeline.rs` to allow the UI to request and retrieve gesture capture sequences.
    *   Updated the core `Pipeline::run` loop to use `tokio::select!` allowing it to listen to `capture_request_rx`.
    *   When a capture request is received, the pipeline waits for the next gesture to finish and sends back the result instead of recognizing and triggering an action.
    *   Cancellations simply send a `()` over a oneshot channel to abort the waiting thread in the handler, ensuring UI responsiveness.

4.  **Frontend:**
    *   Created a self-contained vanilla HTML/JS/CSS frontend in `assets/index.html`. No build steps or frameworks are needed.
    *   Structured into multiple views using standard tab switching.
    *   Used HTML5 `<canvas>` to accurately scale and render raw gesture points when a user is recording a new gesture.
    *   The WebSocket reconnects every 2 seconds if the connection to the daemon drops.

5.  **Dependencies:**
    *   Added `futures-util` to allow working seamlessly with the split `WebSocket` stream (handling sink and stream concurrently).
    *   Enabled the `ws` feature on `axum`.

## Acceptance Criteria Checklist
- [x] `cargo check` passes
- [x] Config UI accessible on `localhost:9876`
- [x] Gesture list loaded
- [x] Recording end-to-end works
- [x] Deletion works
- [x] Settings save and persist
- [x] Documented in `task11.md`