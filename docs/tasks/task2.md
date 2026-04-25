# Task 2 — Implementation Notes: `src/types.rs`

Ambiguities encountered and executive decisions made during creation of the shared types module.

---

## 1. `VirtualKey` — enum vs. newtype string

**Ambiguity:** The architecture shows `VirtualKey` used in `ActionRequest` and `TriggerConfig`, but never defines what `VirtualKey` itself is. It appears in config as plain strings (`key = "F1"`, `modifiers = ["Ctrl"]`), which implies serialisation, but the spec is silent on the type's representation.

**Decision:** Defined as a `String` newtype — `pub struct VirtualKey(pub String)` — rather than a large enum of every possible virtual key code. This keeps config files human-readable without maintaining an exhaustive variant list, and matches the string literals visible in the TOML examples. If the team later wants compile-time validation of key names, the newtype wrapper makes it easy to add a `TryFrom<String>` without changing any downstream code.

---

## 2. `GestureTemplate` — what fields to include

**Ambiguity:** The architecture defines `GestureTemplate` only by its usage in the `GestureRecognizer` trait signature (`templates: &[GestureTemplate]`) and the gesture profile TOML schema. The struct itself is never explicitly declared in the Key Types section alongside `InputEvent`, `GestureCapture`, etc.

**Decision:** Derived the fields (`name`, `template_points`, `algorithm`) from the `[gestures.pattern]` block in the TOML schema. The `raw` recording data (`points`, `timestamps`) was intentionally left out — that belongs in `GestureCapture`, which is already the type for raw-recorded data. `GestureTemplate` was treated as the *processed* form that the recogniser actually operates on.

---

## 3. Which types need `Serialize` / `Deserialize`

**Ambiguity:** The task said "types used in config should also derive Serialize, Deserialize" but did not enumerate exactly which types those are.

**Decision:** Applied the following reasoning:

| Type | Serde | Rationale |
|---|---|---|
| `InputEvent` | ✗ | Runtime-only; never persisted or sent over wire |
| `InputEventType` | ✗ | Same as above |
| `MouseButton` | ✓ | Appears in `TriggerConfig` TOML (`first = "Left"`) |
| `GestureCapture` | ✓ | Stored in gesture profile TOML (`[gestures.raw]`) and sent over WebSocket IPC |
| `GestureMatch` | ✓ | Sent over WebSocket IPC (`capture_result` message) |
| `GestureTemplate` | ✓ | Stored in gesture profile TOML and round-tripped through config UI |
| `VirtualKey` | ✓ | Appears in `ActionRequest` config and IPC |
| `ActionRequest` | ✓ | Stored in gesture profile TOML and sent over WebSocket IPC |

---

## 4. `ActionRequest` serde tag strategy

**Ambiguity:** The TOML schema shows `action = { type = "key_press", key = "F1" }`, implying a tagged enum, but the architecture doesn't specify how serde should encode the discriminant.

**Decision:** Used `#[serde(tag = "type", rename_all = "snake_case")]` (internally tagged). This produces `{ "type": "key_press", "key": "..." }` in both JSON (IPC) and the inline TOML table format, matching the examples in the architecture doc exactly.

---

## 5. `TriggerConfig` — omitted from `types.rs`

**Ambiguity:** `TriggerConfig` is defined in the architecture's Trigger System section and references `MouseButton` and `VirtualKey` from types. It's not listed in the Key Types section but is clearly a shared type.

**Decision:** Left it out of `types.rs` for now. `TriggerConfig` is semantically a *config* type, not a *pipeline data-flow* type. It will live in `config.rs` alongside the rest of the config schema, where it can import `MouseButton` and `VirtualKey` from `types`. This avoids a circular concern where `types.rs` starts absorbing config-layer concerns.

---

## 6. `PartialEq` / `Eq` on `MouseButton` and `VirtualKey`

**Ambiguity:** The task only specified `Debug` and `Clone`. Neither the architecture nor the conventions mention equality derives.

**Decision:** Added `PartialEq + Eq` to both `MouseButton` and `VirtualKey` because:
- `MouseButton` will be compared in the trigger state machine (`button == first_button`).
- `VirtualKey` will likely be compared when deduplicating modifier lists.

These are obvious functional requirements implied by the architecture, not gold-plating.

---

## Addendum — Architect Review (2026-04-23)

All 6 decisions accepted with no changes to source files.

### Confirmed: VirtualKey as String newtype
The downstream usage in `config.rs` (task 3) confirmed this was correct — key names flow through TOML as plain strings without any friction.

### Confirmed: TriggerConfig belongs in config.rs
Task 3 implemented `TriggerConfig` cleanly in `config.rs` importing `MouseButton` and `VirtualKey` from `types`. The boundary held exactly as intended — no circular concerns.

### Confirmed: GestureTemplate excludes raw data
Task 3 reinforced this by giving `GestureConfig` (the TOML schema type) a `raw: GestureCapture` field separately from the `pattern: GesturePatternConfig` field. Clean separation of raw-recorded vs processed-template data throughout.

### Confirmed: ActionRequest serde strategy
Task 3 created a parallel `ActionConfig` in `config.rs` using the same `#[serde(tag = "type", rename_all = "snake_case")]` strategy. Both types produce identical JSON/TOML representations, which validates the approach. A `From`/`Into` conversion between them is the planned bridge.
