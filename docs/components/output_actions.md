# Component: Output Actions

## Overview

Output actions are the final stage of the pipeline. When the gesture engine produces a successful match, the pipeline looks up the bound action from the gesture config and executes it. Each action type is an implementation of the `OutputAction` trait.

## Interface

```rust
pub trait OutputAction: Send + 'static {
    /// Execute the action.
    fn execute(&self) -> Result<()>;

    /// Human-readable name for logging
    fn name(&self) -> &str;
}
```

### Contract

- `execute()` should be fast and non-blocking. Keyboard simulation via `SendInput` is effectively instant.
- `execute()` must be idempotent-safe — the pipeline may call it once per recognized gesture, but the action itself should not depend on being called exactly once.
- Errors should be returned (not panicked) so the pipeline can log and continue.

## Action Configuration

Actions are defined per-gesture in the TOML config. The `type` field determines which `OutputAction` implementation to use:

```toml
# Simple keypress
action = { type = "key_press", key = "F1" }

# Keypress with modifiers
action = { type = "key_press", key = "G", modifiers = ["Ctrl", "Shift"] }

# Future: mouse click
action = { type = "mouse_click", button = "Middle" }

# Future: execute code/command
action = { type = "exec", command = "notepad.exe" }
```

### Action Deserialization

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ActionConfig {
    #[serde(rename = "key_press")]
    KeyPress {
        key: String,
        #[serde(default)]
        modifiers: Vec<String>,
    },
    // Future variants:
    // MouseClick { button: String },
    // Exec { command: String },
}
```

The `ActionConfig` enum is deserialized from TOML/JSON config. A factory function converts it into a `Box<dyn OutputAction>`:

```rust
pub fn create_action(config: &ActionConfig) -> Result<Box<dyn OutputAction>> {
    match config {
        ActionConfig::KeyPress { key, modifiers } => {
            let vk = parse_virtual_key(key)?;
            let mods = modifiers.iter().map(|m| parse_virtual_key(m)).collect::<Result<Vec<_>>>()?;
            Ok(Box::new(KeyPressAction { key: vk, modifiers: mods }))
        }
    }
}
```

## Implementation: Keyboard Simulation (v1)

### How It Works

Uses the Win32 `SendInput` API to simulate keyboard input. This works system-wide, including in games.

### Execution Flow

```
1. Press modifier keys (if any): Ctrl, Shift, Alt
   - SendInput with KEYEVENTF_KEYDOWN for each modifier
2. Press and release the main key
   - SendInput with KEYEVENTF_KEYDOWN
   - SendInput with KEYEVENTF_KEYUP
3. Release modifier keys (in reverse order)
   - SendInput with KEYEVENTF_KEYUP for each modifier
```

All key events should be batched into a single `SendInput` call for atomicity:

```rust
pub struct KeyPressAction {
    pub key: u16,            // Virtual key code
    pub modifiers: Vec<u16>, // Modifier virtual key codes
}

impl OutputAction for KeyPressAction {
    fn execute(&self) -> Result<()> {
        let mut inputs: Vec<INPUT> = Vec::new();

        // Press modifiers
        for &modifier in &self.modifiers {
            inputs.push(make_key_input(modifier, false)); // keydown
        }

        // Press and release main key
        inputs.push(make_key_input(self.key, false));  // keydown
        inputs.push(make_key_input(self.key, true));   // keyup

        // Release modifiers (reverse order)
        for &modifier in self.modifiers.iter().rev() {
            inputs.push(make_key_input(modifier, true)); // keyup
        }

        // Send all at once
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent != inputs.len() as u32 {
            return Err(anyhow!("SendInput only sent {}/{} events", sent, inputs.len()));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "key_press"
    }
}
```

### Virtual Key Mapping

The config uses human-readable key names. These must be mapped to Windows virtual key codes:

| Config string | VK code | Notes |
|--------------|---------|-------|
| `"A"` - `"Z"` | `0x41` - `0x5A` | Letters |
| `"0"` - `"9"` | `0x30` - `0x39` | Numbers |
| `"F1"` - `"F24"` | `VK_F1` - `VK_F24` | Function keys |
| `"Space"` | `VK_SPACE` | |
| `"Enter"` | `VK_RETURN` | |
| `"Tab"` | `VK_TAB` | |
| `"Escape"` | `VK_ESCAPE` | |
| `"Ctrl"` | `VK_CONTROL` | Modifier |
| `"Shift"` | `VK_SHIFT` | Modifier |
| `"Alt"` | `VK_MENU` | Modifier |
| `"Left"`, `"Right"`, `"Up"`, `"Down"` | `VK_LEFT`, etc. | Arrow keys |

The parser should be case-insensitive and support aliases (e.g., "ctrl" = "Ctrl" = "CTRL").

### Win32 API Calls

| API | Purpose |
|-----|---------|
| `SendInput` | Synthesize keyboard events |
| `INPUT` struct | Describes a keyboard/mouse/hardware input event |
| `KEYBDINPUT` | Keyboard-specific input data |
| `KEYEVENTF_KEYUP` | Flag indicating key release (absence = key press) |
| `KEYEVENTF_EXTENDEDKEY` | Flag for extended keys (arrows, numpad, etc.) |

### Anti-Cheat Considerations

`SendInput` is the standard Windows API for input simulation. Some games/anti-cheat may:
- Block `SendInput` from non-elevated processes (run as admin if needed)
- Detect synthesized input via `LLKHF_INJECTED` flag in hook data
- Use hardware-level input detection that ignores `SendInput`

For most games (including AoE2), `SendInput` works fine. If issues arise, future output modules could explore hardware-level simulation via driver interfaces.

## Future Implementations

### Mouse Click Action

Simulate mouse button clicks using `SendInput` with `MOUSEINPUT` instead of `KEYBDINPUT`. Would support left/right/middle click, double-click, and click-and-hold.

### Code Execution Action

Run an arbitrary command or script. Would need careful sandboxing considerations:
- Configurable working directory
- Timeout
- Environment variable access
- Possibly restricted to a whitelist of commands

## Tasks

### v1: Keyboard Simulation

- [ ] Create `src/output/mod.rs` with `OutputAction` trait definition and `ActionConfig` enum
- [ ] Create `src/output/keyboard.rs` with `KeyPressAction` struct
- [ ] Implement virtual key name parser (string → VK code, case-insensitive)
- [ ] Implement `make_key_input` helper (VK code → `INPUT` struct)
- [ ] Handle extended key flag for arrow keys, numpad, etc.
- [ ] Implement `execute()`: batch modifier press + key press/release + modifier release into single `SendInput`
- [ ] Implement `create_action()` factory function
- [ ] Unit tests: virtual key parser handles all documented key names
- [ ] Unit tests: modifier ordering is correct (press forward, release reverse)
- [ ] Integration test: simulate a keypress, verify it was received (e.g., via `GetAsyncKeyState`)
