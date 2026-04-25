# Task 8: CLI Capture Mode & Unified Trigger Refinements

## Overview
Implemented a CLI capture mode to record gestures without requiring a web frontend. The user can invoke QuickDraw with the `--capture` argument, perform a gesture, and save it to their configuration file.

Additionally, implemented a major architectural refinement to the trigger system, unifying mouse and keyboard triggers into a single, generic, case-insensitive, and nomenclature-consistent system.

## Decisions

1. **CLI Argument Parsing**: Added basic argument parsing in `src/main.rs`. 
   - If `--capture <name> <action>` is provided, it enters capture mode.
   - Example: `quickdraw --capture "swipe-right" "key:F1"`.

2. **Pipeline modifications (`src/pipeline.rs`)**:
   - **`GestureAccumulator`**: Extracted the point/timestamp tracking logic into a new struct. This allowed sharing the state machine logic between `run()` and the new `capture_one()` method.
   - **`capture_one`**: Handles the full recording lifecycle: starts raw input, drives the trigger detector, normalizes points into a template, and persists to TOML.

3. **Unified & Consistent Trigger System**:
   - **Consistent Nomenclature**: Refactored `TriggerConfig` to use `key1` and `key2` fields regardless of type (`single` or `combo`). This prevents having to rename fields in `config.toml` when switching trigger modes.
   - **Renamed Mouse Buttons**: Mapped mouse buttons to standard `Mouse1` through `Mouse5` names.
   - **Case-Insensitive Matching**: Implemented `matches_key` in `pipeline.rs` using `eq_ignore_ascii_case`. Users can now specify triggers like `"Tab"`, `"TAB"`, or `"mouse1"` in their config without issue.
   - **Backward Compatibility**: Used Serde aliases (`first`, `second`, `button`, `Left`, `Right`, etc.) to ensure older configuration files continue to load correctly into the new system.

4. **Gesture Detection Tolerance (1D Bias Fix)**:
   - **Fix**: Implemented proportional (uniform) scaling in `src/gesture/dollar_one.rs` for 1D shapes. If a shape's shortest dimension is <= 30% of its longest dimension, it scales uniformly, preserving straight line gestures perfectly and removing the diagonal bias.

## Accepted Trigger Keys

Below is the list of keys and buttons that can be used in your `[trigger]` configuration:

### Mouse Buttons
- `Mouse1` (Left Click)
- `Mouse2` (Right Click)
- `Mouse3` (Middle Click)
- `Mouse4` (Side Button 1)
- `Mouse5` (Side Button 2)

### Keyboard Keys
- **Alphanumeric**: `A` through `Z`, `0` through `9`.
- **Function Keys**: `F1` through `F24`.
- **Control**: `Shift`, `Ctrl`, `Alt`, `Win`, `CapsLock`, `NumLock`, `ScrollLock`.
- **Navigation**: `Backspace`, `Tab`, `Enter`, `Space`, `Esc`, `PageUp`, `PageDown`, `Home`, `End`, `Insert`, `Delete`.
- **Arrows**: `Left`, `Right`, `Up`, `Down`.
- **Numpad**: `Num0` through `Num9`.

*Note: All keys are case-insensitive (e.g. `TAB` and `tab` both work).*

## Acceptance Criteria
- [x] `cargo run -- --capture "swipe-right" "key:F1"` compiles and runs
- [x] Nomenclature is consistent (`key1`, `key2`) across all trigger types
- [x] Unified trigger system handles Mouse and Keyboard keys interchangeably
- [x] Triggers are case-insensitive and order-independent
- [x] Mouse buttons renamed to `Mouse1`..`Mouse5`
- [x] Task decisions and accepted keys documented in docs/tasks/task8.md

---

## Addendum — Architect Review (2026-04-25)

All decisions accepted. This task did significantly more than the original scope — the trigger refactor in particular is a meaningful architectural improvement.

### Confirmed: GestureAccumulator extraction
Pulling the point/timestamp accumulation into its own struct and sharing it between `run()` and `capture_one()` is the right call. It eliminates the divergence risk where two code paths accumulate differently and produce subtly different results.

### Confirmed: Unified string-based TriggerConfig
Changing from `ButtonCombo { first: MouseButton, second: MouseButton }` to `Combo { key1: String, key2: String }` is a genuine improvement. String keys allow keyboard modifiers and mouse buttons to be treated uniformly, the TOML schema is simpler, and the serde aliases preserve backward compatibility. ARCHITECTURE.md has been updated to reflect this.

### Confirmed: 1D bias fix in dollar_one.rs
The proportional scaling fallback for degenerate bounding boxes (shortest dimension ≤ 30% of longest) is the right fix for straight-line gestures like swipe-left/right/up/down. Without it, the scale step stretches horizontal lines into squares which breaks recognition. This fix was not in the original spec and should be noted as an improvement to the algorithm.

### Note: accepted trigger key table
The accepted key table in this file is the canonical reference for valid trigger config values. Task documentation for future trigger changes should update this table.
