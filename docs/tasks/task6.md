# Keyboard Simulation Implementation Decisions

During the implementation of the `KeyPressAction` and `OutputAction` trait in `src/output/keyboard.rs` and `src/output/mod.rs`, several technical decisions were made to handle unspecified details and the specific requirements of the `windows` crate.

## 1. Virtual Key Mapping and Parsing
**Issue:** The specification provided a table of common keys but didn't exhaustively list all mappings, particularly for standard alphanumeric characters, and how to handle user aliases (e.g., "ESC" vs "Escape").
**Decision:** I implemented a robust `parse_virtual_key` function. For alphanumeric keys (length 1), it explicitly checks `is_ascii_alphanumeric()` and casts the character to `u16`. This perfectly aligns with Windows virtual key codes where `'A'` is `0x41` and `'0'` is `0x30`. I also added several common aliases ("CONTROL" alongside "CTRL", "RETURN" alongside "ENTER", "ESC" alongside "ESCAPE") to ensure the configuration is forgiving and user-friendly.

## 2. The `windows` Crate `INPUT` Union Structure
**Issue:** The Win32 `INPUT` structure contains a C-style union for mouse, keyboard, and hardware inputs. The `windows-rs` crate represents this union in Rust using generated anonymous wrapper structs.
**Decision:** I utilized the crate's specific generated types: `Anonymous` containing `INPUT_0` which holds the `ki` (`KEYBDINPUT`) field. While this makes the struct initialization slightly verbose (`Anonymous: INPUT_0 { ki: KEYBDINPUT { ... } }`), it correctly satisfies the type system for the Windows API bindings.

## 3. Scan Codes vs. Virtual Keys
**Issue:** `KEYBDINPUT` allows specifying either a virtual key (`wVk`) or a hardware scan code (`wScan`).
**Decision:** I opted to use virtual keys (`wVk`) exclusively and set `wScan` to `0`. Virtual keys are sufficient for the vast majority of games and applications. Attempting to map configuration strings to hardware scan codes introduces significant complexity and varying localization issues. If specific anti-cheat systems block virtual key injection in the future, a separate `DirectInputAction` or scan-code specific action can be implemented.

## 4. `SendInput` Error Handling
**Issue:** The component contract specifies: "Errors should be returned (not panicked) so the pipeline can log and continue."
**Decision:** After calling the `unsafe { SendInput(...) }` function, the implementation checks if the returned integer (the number of events successfully inserted into the keyboard input stream) matches the exact length of the `inputs` vector. If they do not match, it returns an `anyhow` error detailing how many events were sent vs expected, allowing the main pipeline to gracefully recover and log the failure.

## 5. Factory Implementation (`create_action`)
**Issue:** The TOML configuration deserializes into an `ActionConfig` enum. We need a way to instantiate the corresponding `Box<dyn OutputAction>`.
**Decision:** I implemented `pub fn create_action(config: &ActionConfig) -> Result<Box<dyn OutputAction>>` in `src/output/mod.rs`. This cleanly abstracts the parsing of the configuration strings (both the main key and the modifier array) into virtual keys before instantiating the `KeyPressAction` struct, keeping the `execute` method fast and strictly focused on the Windows API calls.

---

## Addendum — Architect Review (2026-04-24)

All 5 decisions accepted with no changes to source files.

### Confirmed: VK mapping approach
Case-insensitive via `.to_uppercase()`, alphanumeric single-char cast, aliases for common variants (ESC/ESCAPE, CTRL/CONTROL, ENTER/RETURN). Clean and user-friendly. The `is_extended_key()` helper correctly handles the navigation keys that require `KEYEVENTF_EXTENDEDKEY`.

### Confirmed: INPUT union syntax
`Anonymous: INPUT_0 { ki: KEYBDINPUT { ... } }` is the correct `windows-rs` generated wrapper pattern. This is a common pain point with the crate — the agent navigated it correctly.

### Confirmed: Virtual keys only
Correct v1 decision. Scan codes introduce localization complexity and vary by hardware layout. If a future anti-cheat requires hardware-level simulation, that belongs in a separate `DirectInputAction` module, not bolted onto this one.

### Confirmed: SendInput error check
Matches the `OutputAction` contract: return `Err`, never panic. The sent-vs-expected count check is the correct way to detect partial failure.

### Confirmed: create_action factory (re: task3 forward-looking note)
The task3 addendum flagged that a `From<ActionConfig> for ActionRequest` conversion would be needed. The agent instead implemented `create_action(config: &ActionConfig) -> Result<Box<dyn OutputAction>>` — which is **better** than a `From` trait impl. `From` cannot return `Result`, and key name parsing is fallible. The factory function handles this correctly. The task3 note is now resolved and superseded.

