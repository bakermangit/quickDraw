// Shared types: InputEvent, GestureCapture, GestureMatch, GestureTemplate, ActionRequest, etc.
// See docs/ARCHITECTURE.md § Key Types.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Input pipeline types
// ---------------------------------------------------------------------------

/// A raw mouse input event from any InputSource.
#[derive(Debug, Clone, PartialEq)]
pub struct InputEvent {
    pub event_type: InputEventType,
    /// Milliseconds, monotonic clock.
    pub timestamp: u64,
}

/// Discriminated variants of an input event.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEventType {
    /// Relative mouse movement.
    MouseMove { dx: i32, dy: i32 },
    /// Mouse button press or release.
    MouseButton { button: MouseButton, pressed: bool },
    /// Keyboard key press or release.
    KeyboardKey { key: VirtualKey, pressed: bool },
}

/// Mouse buttons recognised by QuickDraw.
///
/// Derives `Serialize`/`Deserialize` because it appears in trigger config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

// ---------------------------------------------------------------------------
// Gesture pipeline types
// ---------------------------------------------------------------------------

/// Accumulated mouse data captured during an active gesture recording.
///
/// Stored in gesture profile TOML (the `[gestures.raw]` section), so it
/// derives `Serialize`/`Deserialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureCapture {
    /// Accumulated (x, y) positions relative to the gesture start point.
    pub points: Vec<(f64, f64)>,
    /// Milliseconds elapsed since the gesture started, one entry per point.
    pub timestamps: Vec<u64>,
}

/// Result of a successful gesture recognition pass.
///
/// May be serialised when forwarded over the WebSocket IPC channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureMatch {
    /// Matches the `name` field of the winning `GestureTemplate`.
    pub gesture_id: String,
    /// Normalised confidence score in the range 0.0 – 1.0.
    pub confidence: f64,
}

/// A registered gesture loaded from a gesture-profile TOML file.
///
/// Fully serialisable so the config UI can round-trip templates over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureTemplate {
    /// Human-readable identifier, e.g. `"flick-right"`.  Must be unique within
    /// a profile.
    pub name: String,
    /// Pre-processed template points produced by the recogniser's normalise
    /// step (resampled, scaled, rotated). For Rubine, this holds the raw points
    /// from the capture. Stored so the daemon can skip re-processing on every startup.
    pub template_points: Vec<(f64, f64)>,
    /// Statistical feature vector for recognizers that use it (e.g. Rubine).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<f64>>,
    /// The algorithm that produced (and should match against) these points,
    /// e.g. `"dollar_one"`.
    pub algorithm: String,
}

// ---------------------------------------------------------------------------
// Output pipeline types
// ---------------------------------------------------------------------------

/// A virtual key identifier as a human-readable string (e.g. `"F1"`, `"Ctrl"`).
///
/// Kept as a `String` newtype so config files stay readable without requiring
/// a large enum of every possible VK code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualKey(pub String);

/// An action to be executed by an `OutputAction` implementation.
///
/// Serialisable so actions can be round-tripped through the WebSocket IPC and
/// stored in gesture profile TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)] // Reserved for IPC — will be used when frontend sends actions
pub enum ActionRequest {
    /// Simulate a key press, optionally with modifier keys held.
    KeyPress {
        key: VirtualKey,
        #[serde(default)]
        modifiers: Vec<VirtualKey>,
    },
    // Future variants: MouseClick, CodeExec, etc.
}

/// Commands sent to the main event loop to control the system.
#[derive(Debug, Clone)]
pub enum SystemCommand {
    Quit,
    OpenConfig,
    ReloadEngine,
}
