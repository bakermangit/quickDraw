# Velocity and Path-Length Constraints

## Overview
QuickDraw now supports optional per-gesture constraints on gesture speed and path length. This allows differentiating gestures that have the same shape but are drawn differently (e.g., a fast circle vs. a slow circle, or a large square vs. a small square).

## Design Decisions
The constraints are implemented as optional fields in `GestureConfig`. This approach ensures that:
- Constraints are opt-in and do not affect existing gestures.
- They are evaluated at the gesture level, allowing for fine-grained control.
- Evaluation occurs after successful shape recognition but before action execution.

## Implementation Details
Two new helper functions were added to `src/pipeline.rs`:
- `compute_path_length`: Calculates the total Euclidean distance of the gesture by summing the distances between all consecutive points.
- `compute_speed`: Calculates the average speed of the gesture by dividing the total path length by the final timestamp (duration).

In the gesture completion handler in `src/pipeline.rs`, the matched gesture's constraints are checked against these calculated values. If any constraint (min/max speed or min/max path length) is violated:
1. A warning is logged.
2. The error sound is played.
3. The gesture's action is not executed.

## Configuration
The following fields can be added to a gesture in the profile TOML:
- `min_speed_px_per_ms`: Minimum average speed in pixels per millisecond.
- `max_speed_px_per_ms`: Maximum average speed in pixels per millisecond.
- `min_path_length_px`: Minimum total path length in pixels.
- `max_path_length_px`: Maximum total path length in pixels.

Example:
```toml
[[gestures]]
name = "fast-circle"
# ... (action and pattern)
min_speed_px_per_ms = 2.0
```
