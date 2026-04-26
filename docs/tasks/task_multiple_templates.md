# Task: Multiple Templates Per Gesture

## Objective
Allow a single gesture name to have multiple recorded templates in the configuration file. This improves recognition consistency, as users can record multiple variations of how they draw a specific shape (e.g., a "swipe-right" drawn perfectly horizontal vs. slightly angled).

## Design Decisions
1. **Config Representation**: The `[gestures]` array in the TOML profile natively supports multiple entries. Instead of changing the schema to include nested arrays, we simply allow multiple `[[gestures]]` entries with the *same `name`*.
2. **Pipeline Aggregation**: When loading the configuration (`build_pipeline` in `pipeline.rs`), we construct a flat list of `GestureTemplate` structs. The recognizer iterates through all templates without needing to group them by name.
3. **Best Match Selection**: The `$1` recognizer naturally returns the best match score across all templates. If multiple templates share the same name, the best score for that name is the one that triggers the action.
4. **Action Resolution**: The `Pipeline` stores a `HashMap<String, GestureConfig>` mapping the gesture name to its action, threshold, and sound. If there are duplicate names in the TOML file, the last one defined wins for the `GestureConfig` properties, but *all* templates are pushed to the template array.

## Implementation Details
- No changes to the `$1` recognizer core algorithm were required, as it operates on a flat slice of templates.
- The WebSocket server and web UI were updated to allow saving multiple templates with the same name without deleting the old ones.

## Acceptance Criteria Met
- [x] Can record the same gesture multiple times.
- [x] All recorded templates are saved to the TOML file under the same name.
- [x] Recognition compares the capture against all templates.
- [x] The core action and configuration apply correctly regardless of which specific template variation was matched.
