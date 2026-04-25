# Component: Gesture Engine

## Overview

The gesture engine receives a `GestureCapture` (accumulated mouse positions + timestamps from a completed gesture) and attempts to match it against a library of stored gesture templates. If a match exceeds the configured confidence threshold, it produces a `GestureMatch` that the pipeline uses to look up and execute the bound action.

The engine is composed of two concepts:
1. **Recognizer** — The core algorithm that compares input against templates
2. **Filter** (optional, composable) — Post-recognition checks that can reject matches based on additional criteria (e.g., duration, velocity)

## Interfaces

### GestureRecognizer

```rust
pub trait GestureRecognizer: Send + 'static {
    /// Attempt to recognize a gesture from captured mouse data.
    /// Returns the best match above the confidence threshold, or None.
    fn recognize(
        &self,
        capture: &GestureCapture,
        templates: &[GestureTemplate],
    ) -> Option<GestureMatch>;

    /// Process a raw capture into a template for storage.
    /// Called during gesture recording to generate the processed form.
    fn create_template(&self, capture: &GestureCapture) -> GestureTemplate;

    /// Human-readable name for config
    fn name(&self) -> &str;
}
```

### GestureFilter

```rust
pub trait GestureFilter: Send + 'static {
    /// Post-recognition check. Returns true if the match should be accepted.
    fn accept(
        &self,
        capture: &GestureCapture,
        template: &GestureTemplate,
    ) -> bool;

    fn name(&self) -> &str;
}
```

### GestureTemplate

```rust
pub struct GestureTemplate {
    pub gesture_id: String,
    /// Processed points (algorithm-specific representation)
    pub points: Vec<(f64, f64)>,
    /// Original raw capture (preserved for re-processing)
    pub raw: GestureCapture,
}
```

## Implementation: $1 Recognizer (v1)

The $1 (Dollar One) unistroke recognizer by Wobbrock, Wilson, and Li (2007).

### Algorithm Steps

Given an input gesture (sequence of points) and a set of templates:

#### 1. Resample to N points

Redistribute the gesture's points into N equidistant points along the path. This normalizes for speed variation (fast vs slow drawing).

```
N = 64 (standard)

1. Compute total path length L
2. Ideal spacing I = L / (N - 1)
3. Walk along the path, placing a new point every I distance
4. Linear interpolation between original points as needed
```

#### 2. Rotation normalization — INTENTIONALLY OMITTED

Standard $1 rotates all points so the angle from centroid to first point is 0°, achieving rotation invariance. **QuickDraw deliberately skips this step.**

Reason: direction *is* meaning in this application. An L-shape drawn downward-then-right and an L-shape drawn upward-then-right are different gestures that should map to different actions. Normalizing rotation would make them identical to the recognizer and eliminate the user's ability to define direction-sensitive gestures.

The tradeoff: the user must draw gestures consistently at roughly the same orientation as when they recorded them. This is acceptable — gaming gestures are always performed in the same general direction by design (e.g., "flick right" is always rightward).

#### 3. Scale to bounding box

Scale the gesture to fit a reference square (e.g., 250×250). This provides scale invariance.

```
1. Find bounding box (min/max x and y)
2. Scale all points: new_x = (x - min_x) * size / width, same for y
```

#### 4. Translate to origin

Move the centroid to (0, 0).

```
1. Recompute centroid after scaling
2. Subtract centroid from all points
```

#### 5. Match against templates

For each template, compute the average point-by-point distance. Optionally search over a small angular range (golden section search) for the best rotation alignment.

```
For each template T:
  distance = (1/N) * sum(dist(input[i], T[i]) for i in 0..N)
  score = 1 - distance / (0.5 * sqrt(size^2 + size^2))

Return the template with the highest score above threshold
```

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `N` (resample count) | 64 | Number of equidistant points |
| `size` (scale target) | 250.0 | Reference square dimension |
| `threshold` | 0.80 | Minimum score to accept match |
| `angle_tolerance` | ±15° | Small tolerance during matching to absorb hand wobble, but NOT a full rotation search |

The angle tolerance is intentionally small. It compensates for slight inconsistency in how the user starts a gesture, but preserves direction discrimination.

These should be configurable but with sensible defaults.

### Reference

- Original paper: [Gestures without Libraries, Toolkits or Training: A $1 Recognizer for User Interface Prototypes](http://faculty.washington.edu/woერock/pubs/dollar.pdf)
- Reference implementation: [depts.washington.edu/acelab/proj/dollar](http://depts.washington.edu/acelab/proj/dollar/index.html)

### Key Properties

- **Single template**: Works with just one recorded sample per gesture
- **Direction-sensitive**: Rotation normalization is omitted — an L and a mirrored L are distinct gestures
- **Scale invariant**: Gestures can be drawn at any size
- **Speed invariant**: Resampling normalizes for drawing speed
- **Low false positives**: The point-by-point distance metric and confidence threshold make spurious matches unlikely
- **Small angle tolerance**: ±15° wobble tolerance during matching without sacrificing direction discrimination
- **Not velocity-aware**: Speed/timing information is deliberately normalized away (addressed by composable filters)

## Future Implementations

### Rubine Recognizer

Statistical feature-based approach. Extracts ~13 features from the gesture (initial angle, bounding box diagonal, total length, etc.) and uses linear discriminant analysis. Requires multiple training samples per gesture but natively captures velocity/timing as features.

### Duration Filter

Simple composable filter that checks total gesture duration:

```rust
pub struct DurationFilter {
    pub min_ms: Option<u64>,
    pub max_ms: Option<u64>,
}

impl GestureFilter for DurationFilter {
    fn accept(&self, capture: &GestureCapture, _template: &GestureTemplate) -> bool {
        let duration = capture.timestamps.last().unwrap_or(&0) - capture.timestamps.first().unwrap_or(&0);
        if let Some(min) = self.min_ms { if duration < min { return false; } }
        if let Some(max) = self.max_ms { if duration > max { return false; } }
        true
    }
}
```

### Velocity Profile Filter

More sophisticated filter that compares the velocity curve of the input against the template. Useful for distinguishing gestures that have the same shape but different speed profiles (e.g., "flick right" vs "drag right").

## Tasks

### v1: $1 Recognizer

- [ ] Create `src/gesture/mod.rs` with trait definitions (`GestureRecognizer`, `GestureFilter`, `GestureTemplate`)
- [ ] Create `src/gesture/dollar_one.rs` with `DollarOneRecognizer` struct
- [ ] Implement resampling (redistribute N equidistant points along path)
- [ ] **Skip rotation normalization** (do NOT implement indicative angle rotation — direction is intentional)
- [ ] Implement scaling to reference bounding box
- [ ] Implement translation to origin
- [ ] Implement point-by-point distance scoring
- [ ] Implement small angle search (±15° only) to absorb hand wobble during matching
- [ ] Implement `recognize()`: run all steps, match against templates, return best above threshold
- [ ] Implement `create_template()`: process a raw capture into a storable template
- [ ] Unit tests: resample produces correct number of points, known gesture matches itself
- [ ] Unit tests: scaled versions of same gesture still match (scale invariant)
- [ ] Unit tests: **direction discrimination** — L-shape and mirrored-L do NOT match each other
- [ ] Unit tests: different gestures don't match (false positive check)
- [ ] Unit tests: below-threshold scores return None
