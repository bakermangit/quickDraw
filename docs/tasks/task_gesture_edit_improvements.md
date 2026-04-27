# Task: Gesture Editing & Renaming Improvements

## Objective
Enable editing of the gesture name field in the configuration UI and provide clear visual context in the recording/editing modal.

## Implementation Details

### Frontend (`assets/index.html`)
- **Editable Names**: Removed the `disabled` attribute from the gesture name input field in the `editGesture` function, allowing users to rename existing gestures.
- **Dynamic Modal Titles**:
    - Added an `id="modal-title"` to the modal's `<h2>` element.
    - Updated `startRecording`, `addTemplate`, and `editGesture` to dynamically update the title to "Record Gesture", "Add Template", or "Edit Gesture" respectively.
- **Renaming Logic**:
    - Introduced a global `originalName` variable to track the gesture name before any edits.
    - Updated `saveNewGesture` to send both `old_name` (the original name) and `new_name` (the current value of the input field) when sending an `update_gesture` message.

### Backend (`src/server/handlers.rs`)
- **Protocol Update**: Modified the `ClientMessage::UpdateGesture` enum variant to include both `old_name: String` and `new_name: String`.
- **State Update**: The `UpdateGesture` handler now iterates through the gesture profile and updates the `name` field for all templates matching `old_name`. This ensures that renaming a gesture correctly updates the entire group of associated templates.

## Decisions & Rationale
- **Renaming Support**: Previously, gestures couldn't be renamed because the backend matched updates strictly by name. By passing the `old_name`, we can safely update the name of the gesture group while maintaining its existing templates and actions.
- **UI Consistency**: Updating the modal title to "Edit Gesture" when the user clicks the Edit button removes ambiguity, as the modal previously always said "Record Gesture" even when no recording was taking place.

## Verification
- `cargo check` passes.
- UI correctly displays "Edit Gesture" title.
- Gesture name field is now editable in the modal.
- Renaming a gesture successfully updates the `gestures.toml` file with the new name across all associated templates.
