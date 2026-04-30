# Task: Trace Finesse (Dynamic Stroke Width)

## Implementation Details

"Trace Finesse" improves the aesthetics of the gesture trace overlay by making the stroke width dynamic. The stroke starts at a minimum width and grows as the gesture is drawn, providing a visual indication of the gesture's origin and direction.

### Architecture

- **State Management**: The `TraceOverlay` message loop maintains a `current_stroke_width: f64` variable.
- **Dynamic Growth**:
    - Upon receiving `TraceCommand::Begin`, `current_stroke_width` is initialized to `trace_min_stroke` (if finesse is enabled) or `trace_max_stroke` (if disabled).
    - Upon receiving `TraceCommand::AddPoint`, a new GDI pen is created with the integer value of `current_stroke_width`.
    - `current_stroke_width` is incremented by `trace_growth_rate` after each point, clamped at `trace_max_stroke`.
- **GDI Resource Management**:
    - To support varying stroke widths within a single gesture trace, a new pen is created for each segment.
    - **Crucial**: Each temporary pen is selected into the memory device context, used for `LineTo`, and then immediately deselected and destroyed using `DeleteObject` to prevent GDI handle leaks.

### Configuration

The following fields were added to `GeneralConfig`:
- `trace_finesse_enabled` (bool): Toggle for the dynamic width feature. Default: `false`.
- `trace_min_stroke` (i32): Starting width of the trace. Default: `1`.
- `trace_max_stroke` (i32): Maximum width of the trace (or fixed width if finesse is disabled). Default: `10`.
- `trace_growth_rate` (f64): How much the width increases per added point. Default: `0.2`.

### UI Integration

The web configuration UI (`assets/index.html`) has been updated to include a "Trace Settings" section in the Settings tab. This section provides:
- A checkbox for enabling/disabling Trace Finesse.
- Range sliders for Min Stroke Width, Max Stroke Width, and Growth Rate, with real-time value displays.
- Automatic synchronization with the backend via the existing WebSocket protocol.
