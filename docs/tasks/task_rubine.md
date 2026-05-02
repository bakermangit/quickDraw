# Task: Clean-Room Implementation of the Rubine Recognizer

## Overview
We need to implement a simplified Rubine Recognizer. You previously attempted this on an old branch, but that branch accumulated too much technical debt and merge conflicts. We are doing a clean-room implementation straight onto `main`.

You must implement the 13 dynamic features from the 1991 Rubine paper, but use a normalized distance calculation so that human physical variance does not destroy the confidence score.

## Implementation Steps

### 1. `src/types.rs`
- Add a `features: Option<Vec<f64>>` field to the `GestureTemplate` struct.

### 2. `src/gesture/rubine.rs` (NEW FILE)
Create this file and implement the `GestureRecognizer` trait.
- **Guard Clause:** In `extract_features`, return `[0.0; 13]` if `capture.points.len() < 3` OR `capture.timestamps.len() < capture.points.len()`.
- **f0, f1:** Cosine and sine of the initial angle. **CRITICAL:** Use the vector from `p[0]` to `p[1]`, not `p[2]`.
- **f2, f3:** Length and angle of the bounding box diagonal.
- **f4, f5, f6:** Distance, cosine, and sine of the angle between the first and last points.
- **f7:** Total stroke length.
- **f8, f9, f10:** Total angle traversed, total absolute angle traversed, and sum of squared angle changes.
- **f11:** Maximum speed squared.
- **f12:** Total duration. Use `saturating_sub` to calculate `t[last] - t[first]`.

#### Normalized Mathematical Matching
Do NOT use fixed, un-normalized weights like `0.01` for pixel distances, as they explode the Euclidean distance for human input.
Instead, when comparing `input_features` to `template_features`:
1. Calculate the percentage difference for non-angle features (length, duration): `let diff = (input - template) / template.max(1.0);`
2. Multiply by a tuning weight.
3. Calculate distance and map to a confidence score that allows for ~15% human variance.

### 3. `src/gesture/mod.rs`
- Export the `rubine` module.

### 4. `src/pipeline.rs`
- Add support for the `"rubine"` string in the pipeline recognizer matching.
- **Speed Calculation Fix:** In `compute_speed`, calculate duration using `last.saturating_sub(first)`.
- **Keyboard Hook Fix:** In `build_pipeline()`, if `keyboard_input_method` is `"hook"`, return an explicit `anyhow!` error stating that the hook backend only supports mouse input.

## Convention Enforcement
- **Doctests:** You MUST write a "Happy Path" executable ````rust ` doctest in the `///` docstring of the `RubineRecognizer` struct. Do NOT add doctests to private helper functions.

## Definition of Done
- `[ ]` `RubineRecognizer` is implemented cleanly without any Linux GUI dependencies.
- `[ ]` `cargo test` passes.
- `[ ]` `docs/REPO_MAP.md` has been updated using `cargo run --manifest-path tools/repo_map_generator/Cargo.toml`.
- `[ ]` **Self-Documentation:** Write your design decisions in the notes section below before submitting the PR.

---
## Implementation Notes & Agent Feedback
*(Agent: Write your notes here)*
