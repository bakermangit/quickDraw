# GUI Integration for Velocity and Length Filters

## Overview
The QuickDraw configuration UI now supports configuring and visualizing velocity (speed) and path-length constraints for gestures. This allows users to fine-tune gesture recognition not just based on shape, but also on the dynamics of how they are drawn.

## Design Decisions

### Auto-Calculation and Smart Buffering
To make the configuration process user-friendly, the UI automatically calculates the speed and length of any newly drawn gesture template.
- **Decision**: Automatically enable filters for new gestures.
- **Decision**: Pre-fill min/max inputs with a ±30% buffer (0.7x for min, 1.3x for max). This provides a sensible starting point that accounts for natural variation in drawing speed and size while still providing immediate filtering benefits.

### Contextual Statistics
Users need to know how their recorded templates perform to set accurate filters.
- **Decision**: Display aggregate statistics (Minimum, Maximum, Average) for speed and length in the gesture group header.
- **Decision**: Display specific badges next to each individual template preview showing its unique speed and length.
- **Implementation**: These stats are calculated on-the-fly during UI rendering from the raw point and timestamp data stored in the profile.

### Unified Configuration
Constraints like speed and length are conceptually properties of the *gesture* (the name/action mapping) rather than individual templates.
- **Decision**: Synchronize constraint changes across all templates sharing the same name. When a user edits the speed or length filter for a gesture, the `update_gesture` command propagates these values to every template entry in the backend profile.

## Implementation Details

### JavaScript Helpers
- `calculatePathLength(points)`: Sums Euclidean distances between consecutive points.
- `calculateSpeed(points, timestamps)`: Computes `totalLength / (lastTimestamp - firstTimestamp)`.
- `toggleFilterInputs()`: Manages the visibility of the number inputs based on the "Enable" checkbox states.

### UI Integration
- Added `.badge` and `.filter-section` styles to `assets/index.html`.
- Updated `record-modal` to include the filter controls.
- Updated `renderGestures` to calculate and display the new badges and aggregate stats.
- Updated WebSocket payload handling in `saveNewGesture` and `editGesture` to include the four new optional constraint fields.

### Backend Support
- The `UpdateGesture` message in `src/server/handlers.rs` was expanded to include the four optional `f64` fields, ensuring that GUI changes are persisted back to the TOML profile.
