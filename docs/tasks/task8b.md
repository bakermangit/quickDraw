# Task 8b: Portable Configuration Mode

## Overview
Implemented a "portable mode" for QuickDraw configuration. If a `config.toml` file exists in the same directory as the application executable, the application will use that directory for all its configuration and data (gestures, logs, etc.). Otherwise, it falls back to the standard `%APPDATA%\QuickDraw` directory.

## Decisions

1. **Executable Path Discovery**: Used `std::env::current_exe()` to reliably find the path of the running process.
2. **Priority logic**: The check for a local `config.toml` happens first. This gives the user an explicit way to opt-in to portable mode by simply copying their config next to the exe.
3. **Logging**: Added `tracing::debug!` calls in `get_config_dir` to help with troubleshooting and to confirm which mode is active during development.

## Acceptance Criteria
- [x] `cargo check` passes
- [x] Placing a `config.toml` next to the exe makes the app use that directory
- [x] Removing `config.toml` from the exe directory falls back to AppData cleanly
- [x] Document decisions in `docs/tasks/task8b.md`
