# Task: Implement Simplified Rubine Recognizer

## Overview
The Rubine Recognizer is a statistical gesture recognition algorithm that uses a set of 13 features extracted from the gesture's path, timing, and velocity. Unlike the $1 Unistroke Recognizer, which focuses on shape, Rubine considers the dynamic properties of the gesture.

## Implementation Details

### Feature Extraction
The `RubineRecognizer` extracts 13 features from a `GestureCapture`:
- **f0, f1**: Cosine and sine of the initial angle.
- **f2, f3**: Length and angle of the bounding box diagonal.
- **f4, f5, f6**: Distance, cosine, and sine of the angle between the first and last points.
- **f7**: Total stroke length.
- **f8, f9, f10**: Total angle traversed, total absolute angle traversed, and sum of squared angle changes.
- **f11**: Maximum speed squared.
- **f12**: Total duration.

### Normalization and Matching
Instead of a linear classifier, we use **Weighted Euclidean Distance** for matching. Each feature is multiplied by a normalization weight to ensure that features with different scales (e.g., angles vs. durations) contribute appropriately to the final distance.

**Normalization Weights (`RUBINE_WEIGHTS`):**
- Angles: 1.0
- Distances: 0.01
- Speed squared: 0.1
- Duration (ms): 0.001

### Confidence Score
The distance is converted to a confidence score:
`confidence = 1.0 / (1.0 + distance)`

## Changes
- **src/types.rs**: Added `features` field to `GestureTemplate`.
- **src/gesture/rubine.rs**: Implemented feature extraction, normalization, and matching logic.
- **src/gesture/mod.rs**: Exported the `rubine` module.
- **src/pipeline.rs**: Added support for the "rubine" recognizer in the pipeline.

## Verification
- Unit tests in `rubine.rs` verify feature extraction with dummy data and edge cases (e.g., < 3 points).
- Unit tests verify that exact matches result in a confidence of 1.0 and slight variations maintain high confidence.
