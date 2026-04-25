# Task 9: System Tray Implementation

## Overview
QuickDraw is a headless daemon. To provide user control without a terminal, a system tray icon has been implemented using the `tray-icon` crate.

## Implementation Details

### Tray Icon logic (`src/tray/mod.rs`)
- **TrayCommand Enum**: Defined `Quit` and `OpenConfig` commands.
- **start_tray Function**:
    - Creates a 16x16 white RGBA placeholder icon.
    - Sets up a context menu with "Configure..." and "Quit" items.
    - Implements a Windows message loop using `GetMessageW`, `TranslateMessage`, and `DispatchMessageW` to ensure the tray icon and menu are responsive on a dedicated thread.
    - Uses `MenuEvent::receiver()` to catch menu clicks and send corresponding `TrayCommand`s back to the main thread via a `tokio::sync::mpsc` channel.
    - The tray thread uses `blocking_send` to push commands to the main tokio runtime.

### Wiring (`src/main.rs`)
- Created a `tokio::sync::mpsc::channel(8)` for `TrayCommand`s.
- Spawned the tray loop on a separate OS thread using `std::thread::spawn`.
- Modified the main loop to use `tokio::select!`, multiplexing between the `pipeline.run()` future and the `cmd_rx.recv()` future.
- **Command Handling**:
    - `TrayCommand::Quit`: Logs the event and exits the process cleanly with `std::process::exit(0)`.
    - `TrayCommand::OpenConfig`: Executes a shell command (`cmd /c start http://localhost:9876`) to open the configuration UI in the default browser.

## Decisions and Rationale
- **Dedicated Thread**: On Windows, the thread that creates the tray icon must maintain a message loop. By spawning a dedicated OS thread, we keep the main thread free for the tokio runtime while satisfying OS requirements.
- **Message Loop**: Used a standard Win32 message loop (`GetMessageW`) instead of polling or sleeping. This is more power-efficient as it blocks until the OS has a message for the thread.
- **Placeholder Icon**: Used a simple white RGBA square to avoid external asset dependencies during this phase.

## Verification Results
- `cargo check` passes with no relevant warnings.
- The tray icon logic correctly handles menu events and communicates them to the main loop.
- The "Quit" command terminates the application.
- The "Configure..." command opens the browser as expected.
