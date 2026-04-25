# Task 8a: Per-Gesture Confidence Threshold Overrides

## Overview
Added support for optional per-gesture confidence thresholds in the gesture profile TOML. This allows users to fine-tune the recognition sensitivity for specific gestures independently of the global `confidence_threshold` floor.

## Decisions

1. **Config Schema Modification (`src/config.rs`)**:
   - Added `confidence_threshold: Option<f64>` to the `GestureConfig` struct.
   - This field is optional and defaults to `None`. If missing, the pipeline falls back to the global `general.confidence_threshold`.

2. **Recognizer Decoupling (`src/gesture/dollar_one.rs`)**:
   - Removed the threshold filtering logic from `DollarOneRecognizer::recognize()`.
   - The recognizer now always returns the best match found (if any), regardless of the confidence score.
   - This shift allows the pipeline to make decisions based on both global and per-gesture settings.
   - Cleaned up the recognizer API by removing the `threshold` field and updating the constructor.

3. **Pipeline Enforcement (`src/pipeline.rs`)**:
   - Updated the `Pipeline` struct to store `gesture_configs: HashMap<String, GestureConfig>`.
   - Refactored `build_pipeline` to populate this map during initialization.
   - In the `GestureComplete` handler, the pipeline now:
     - Retrieves the matched gesture's configuration.
     - Determines the effective threshold (per-gesture override or global default).
     - Executes the action only if the match confidence meets or exceeds the effective threshold.
     - Logs a warning if a match is rejected due to low confidence.

## Acceptance Criteria
- [x] `cargo check` passes
- [x] Per-gesture `confidence_threshold` in the profile is respected
- [x] Gestures without a specific threshold use the global default
- [x] Unit tests in `dollar_one.rs` updated to reflect the removal of threshold-based filtering
- [x] Decisions documented in `docs/tasks/task8a.md`

---

## Addendum — Architect Review (2026-04-25)

All decisions accepted. The architecture is cleaner after this task than it was before.

### Confirmed: Threshold moved out of recognizer
Removing the threshold filter from `DollarOneRecognizer::recognize()` is the right layering. The recognizer's job is to find the best match; the pipeline's job is to decide whether that match is good enough. This also means future features like "show confidence in the UI even for rejected gestures" are trivially possible — the confidence value is always available now.

### Confirmed: gesture_configs HashMap in Pipeline
Storing `GestureConfig` by name in the pipeline gives the GestureComplete handler access to per-gesture settings without re-loading the profile file. This also positions us well for future per-gesture settings beyond threshold (e.g., per-gesture trigger, per-gesture cooldown).
