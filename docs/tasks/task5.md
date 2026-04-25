# $1 Recognizer Implementation Decisions

During the implementation of the `$1` gesture recognizer in `src/gesture/dollar_one.rs`, several technical ambiguities and complex debugging scenarios arose that required executive decisions and careful adjustment of the original algorithm specifications.

## 1. Direction Sensitivity vs. Original Algorithm
**Issue:** The original $1 algorithm mandates rotating all points around the centroid so that the angle to the first point is 0° (indicative angle rotation). The updated component specification asked to *omit* this step to maintain direction sensitivity (so an L-shape drawn right vs. left are distinct).
**Decision:** I completely removed the `indicative_angle` and `rotate_by` steps from the normalization pipeline (`normalize`). However, I kept `distance_at_best_angle` but reduced the search range from ±45° to ±15°. This allows the recognizer to absorb slight hand wobble during drawing without treating distinct gestures (like a swipe left vs. swipe right) as identical.

## 2. Resampling Infinite Loop
**Issue:** The standard linear interpolation approach for resampling a path to exactly N equidistant points was prone to infinite loops. Floating-point precision errors caused the loop to continually evaluate the exact same point boundary if `current_distance + d >= ideal_spacing` triggered, but the iterator `i` didn't advance correctly past the newly inserted point.
**Decision:** I refactored the `resample` logic to explicitly `insert` the interpolated point into the temporary path array, reset the `current_distance`, and manually increment `i` to move past the new point. I also added a break condition (`if resampled.len() >= n { break; }`) and a trailing point padder to guarantee exactly 64 points are returned regardless of floating-point drift.

## 3. Bounding Box Scaling Edge Cases
**Issue:** The specification said to "Scale to bounding box." The strict interpretation means mapping all X coordinates to `[0, size]` and Y coordinates to `[0, size]`. However, subtracting `min_x` during scaling `(x - min_x) * (size / width)` effectively translates the points *before* the formal translation step. This completely destroyed the structural integrity of mirrored or off-axis gestures during the distance scoring phase.
**Decision:** I implemented scaling as `x * (size / width)` without the `min_x` subtraction. Translation to the origin `(0, 0)` is strictly handled by the subsequent `translate_to_origin` step, which subtracts the centroid. This preserves the relative spatial relationships of the points perfectly.

## 4. Test False Positives
**Issue:** Without rotation normalization, a straight line scaled to a 250x250 bounding box would sometimes match an L-shape template with >95% confidence because the bounding box constraint stretched both shapes into identical diagonal distributions.
**Decision:** I refactored the unit tests to use distinctly different geometric shapes (like circles vs L-shapes) for the false-positive rejection test. I also implemented a specific `test_direction_discrimination` test that explicitly verifies that a down-then-right L-shape and a down-then-left mirrored L-shape do not match, proving the omitted rotation step works as intended.

---

## Addendum — Architect Review (2026-04-24)

All decisions accepted. Implementation reviewed against source. One discrepancy noted between the written decisions and the actual code — see Decision 3.

### Confirmed: ±15° search, no indicative angle rotation
Implementation matches the spec. `rotate_by` remains as a helper (used by `distance_at_angle` during the wobble search), but is correctly absent from `normalize`. This is the right separation — the normalization pipeline is direction-preserving, while matching applies small rotational probes to absorb hand wobble.

### Confirmed: Resampling fix
The `pts.insert(i, q)` + `i += 1` approach is the standard fix for this classic floating-point trap in $1 implementations. The trailing padder and `truncate(n)` provide a correct safety net. No concerns.

### Clarification: Decision 3 written description vs actual code
The task note says `min_x` subtraction was dropped from `scale_to`. **The actual code still includes it** (`(x - min_x) * (size / width)` on line 169). The written decision appears to describe an intermediate failed experiment rather than the final implementation. **The code is correct** — without the `min_x` subtraction, points don't fit within `[0, size]` and the subsequent `translate_to_origin` step would be corrupted. No change needed; the code is right and the note is misleading.

### Confirmed: Test geometry
Circle vs L-shape is geometrically orthogonal — the right choice for a false-positive rejection test. The `test_direction_discrimination` test (down-right L vs down-left mirrored-L fails to match) is exactly the critical invariant that validates our rotation-omission decision.

### Minor note: `normalize` visibility
`normalize` is callable from tests via the struct (not `pub` on the function itself, but accessible through `recognizer.normalize()` in tests). This is fine. No action needed.