# QuickDraw

A system-wide mouse gesture recognition engine for Windows, built in Rust. Designed primarily for gaming (works in exclusive fullscreen), but applicable to any context.

QuickDraw runs as a lightweight background daemon that captures mouse input, recognizes gestures, and executes configurable actions (keyboard input, mouse clicks, code execution).

## Key Features

- **System-wide**: Works in exclusive fullscreen games via Raw Input
- **Modular**: Swap input capture methods, recognition algorithms, and output actions independently
- **Lightweight**: No overlay, no GUI — just a tray icon and optional web-based config
- **Anti-cheat friendly**: Multiple input capture backends (Raw Input, hooks, polling) for compatibility
- **Configurable triggers**: M1+M2 combo, keyboard modifiers, or custom triggers

## Architecture

```
Input Source ──► Gesture Engine ──► Output Action
(Raw Input)      ($1 Recognizer)    (Keyboard Sim)
(Hooks)          (Rubine)           (Mouse Click)
(Polling)        (+ Velocity Filter)(Code Exec)
```

See [DESIGN_OVERVIEW.md](docs/DESIGN_OVERVIEW.md) and [ARCHITECTURE.md](docs/ARCHITECTURE.md) for full details.

## Project Status

**Pre-development** — Documentation and architecture phase.

### v1 Scope
- [x] Documentation and architecture design
- [ ] Raw Input capture
- [ ] $1 gesture recognizer
- [ ] Keyboard output simulation
- [ ] TOML configuration
- [ ] System tray icon
- [ ] WebSocket IPC + web config UI

## Building

```bash
cargo build --release
```

## Configuration

Gesture profiles are stored in TOML format. See `config.example.toml` for reference.

## Documentation

- [Design Overview](docs/DESIGN_OVERVIEW.md) — Vision, goals, constraints
- [Architecture](docs/ARCHITECTURE.md) — Technical design, interfaces, data flow
- [Conventions](docs/CONVENTIONS.md) — Code patterns and contribution guidelines
- [Input Capture](docs/components/input_capture.md) — Input source module
- [Gesture Engine](docs/components/gesture_engine.md) — Recognition algorithms
- [Output Actions](docs/components/output_actions.md) — Action execution module

## License

TBD
