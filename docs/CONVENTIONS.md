# Conventions

## Rust Patterns

### Data Types

Use plain structs with `pub` fields. No getters or setters unless there's a genuine invariant to protect.

```rust
// Good
pub struct GestureCapture {
    pub points: Vec<(f64, f64)>,
    pub timestamps: Vec<u64>,
}

// Bad — unnecessary encapsulation
pub struct GestureCapture {
    points: Vec<(f64, f64)>,
}
impl GestureCapture {
    pub fn points(&self) -> &[(f64, f64)] { &self.points }
}
```

### Error Handling

Use `anyhow::Result` for application errors (main, pipeline, config loading). Use `thiserror` for library-level errors in module implementations where callers need to match on specific variants.

```rust
// Application-level: anyhow
fn load_config(path: &Path) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read config file")?;
    // ...
}

// Module-level: thiserror when callers need to match
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("Failed to register raw input device: {0}")]
    RegistrationFailed(#[from] windows::core::Error),
    #[error("Input source already running")]
    AlreadyRunning,
}
```

Always add context to errors using `.context()` or `.with_context(||)`. Never use `.unwrap()` outside of tests.

### Trait Design

Traits define module interfaces. Keep them minimal — only the methods that every implementation must provide.

```rust
// Good — minimal interface
pub trait InputSource: Send + 'static {
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn name(&self) -> &str;
}

// Bad — too specific, leaks implementation details
pub trait InputSource {
    fn register_raw_input_device(&self) -> Result<()>;
    fn set_hook(&self) -> Result<()>;
}
```

### Static vs Dynamic Dispatch

Prefer generics (static dispatch) for internal functions. Use `dyn Trait` only at the pipeline assembly boundary where the implementation is chosen at runtime from config.

```rust
// Internal function — static dispatch
fn resample_points<I: Iterator<Item = (f64, f64)>>(points: I, n: usize) -> Vec<(f64, f64)> {
    // ...
}

// Pipeline assembly — dynamic dispatch (runtime choice from config)
fn create_recognizer(config: &Config) -> Box<dyn GestureRecognizer> {
    // ...
}
```

### Concurrency

Use channels for inter-component communication. No shared mutable state. No `Arc<Mutex<T>>` unless absolutely unavoidable (and document why if you do).

```rust
// Good — message passing
let (tx, rx) = tokio::sync::mpsc::channel(64);
input_source.start(tx)?;

// Avoid — shared mutable state
let state = Arc::new(Mutex::new(PipelineState::default()));
```

## Naming

| Item | Convention | Example |
|------|-----------|---------|
| Crate | `snake_case` | `quickdraw` |
| Modules | `snake_case` | `raw_input`, `dollar_one` |
| Types | `PascalCase` | `GestureCapture`, `InputEvent` |
| Functions | `snake_case` | `recognize_gesture`, `start_capture` |
| Constants | `SCREAMING_SNAKE` | `DEFAULT_CONFIDENCE_THRESHOLD` |
| Config keys | `snake_case` | `input_method`, `confidence_threshold` |
| Gesture names | `kebab-case` | `"flick-right"`, `"L-shape"` |
| Trait names | `PascalCase`, describe capability | `InputSource`, `GestureRecognizer` |

## Logging

Use the `tracing` crate (not `log`). Levels:

- `error!` — Something failed and an action couldn't be completed
- `warn!` — Something unexpected but recoverable (e.g., unrecognized gesture)
- `info!` — Major lifecycle events (daemon start/stop, config reload)
- `debug!` — Per-gesture recognition results, trigger state changes
- `trace!` — Per-event data (individual mouse moves — very noisy)

Default config level: `warn`. Use `debug` during development.

```rust
use tracing::{info, debug, warn};

info!("QuickDraw daemon started, input method: {}", config.general.input_method);
debug!(gesture = name, confidence = %score, "Gesture recognized");
warn!("No matching gesture found for capture ({} points)", capture.points.len());
```

## Adding a New Module

### New Input Source

1. Create `src/input/your_source.rs`
2. Implement `InputSource` trait
3. Add the module to `src/input/mod.rs`
4. Add a match arm in `build_pipeline()` in `src/main.rs` (or `src/pipeline.rs`)
5. Document in `docs/components/input_capture.md`

### New Gesture Recognizer

1. Create `src/gesture/your_recognizer.rs`
2. Implement `GestureRecognizer` trait
3. Add the module to `src/gesture/mod.rs`
4. Add a match arm in `build_pipeline()`
5. Document in `docs/components/gesture_engine.md`

### New Output Action

1. Create `src/output/your_action.rs`
2. Implement `OutputAction` trait
3. Add the module to `src/output/mod.rs`
4. Register in action deserialization (config.rs — the `action.type` field)
5. Document in `docs/components/output_actions.md`

### New Gesture Filter

1. Create `src/gesture/filters/your_filter.rs`
2. Implement `GestureFilter` trait
3. Add to the filter chain in pipeline config
4. Document in `docs/components/gesture_engine.md`

## Dependencies (Expected v1)

| Crate | Purpose |
|-------|---------|
| `windows` | Win32 API bindings (Raw Input, SendInput) |
| `tokio` | Async runtime, channels, task spawning |
| `serde` + `serde_json` | Serialization for IPC messages |
| `toml` | Config file parsing |
| `anyhow` + `thiserror` | Error handling |
| `tracing` + `tracing-subscriber` | Structured logging |
| `tray-icon` | System tray icon |
| `tungstenite` or `tokio-tungstenite` | WebSocket server |
| `axum` | HTTP server (serves config UI static files) |

## Testing

- Unit tests for pure logic (gesture recognition, config parsing, trigger state machine)
- Integration tests for pipeline assembly
- No mocking frameworks — use trait implementations for test doubles
- Test files live alongside source: `#[cfg(test)] mod tests { ... }` in each file

## Git

- Commit messages: imperative mood, concise (`Add raw input source`, `Fix trigger state machine edge case`)
- One logical change per commit
- `main` branch should always compile
