use std::f64::consts::PI;

use crate::types::{GestureCapture, GestureMatch, GestureTemplate};
use super::GestureRecognizer;

/// Normalization weights for the 13 Rubine features.
/// Based on typical ranges to ensure no single feature dominates the Euclidean distance.
const RUBINE_WEIGHTS: [f64; 13] = [
    1.0,   // f0: Cosine of initial angle
    1.0,   // f1: Sine of initial angle
    0.01,  // f2: Length of bounding box diagonal
    1.0,   // f3: Angle of bounding box diagonal
    0.01,  // f4: Distance between first and last points
    1.0,   // f5: Cosine of angle between first and last points
    1.0,   // f6: Sine of angle between first and last points
    0.01,  // f7: Total stroke length
    1.0,   // f8: Total angle traversed
    1.0,   // f9: Total absolute angle traversed
    1.0,   // f10: Sum of squared angle changes
    0.1,   // f11: Maximum speed squared
    0.001, // f12: Total duration
];

/// A simplified Rubine Gesture Recognizer.
///
/// This recognizer extracts 13 dynamic features from a gesture capture, including
/// initial angle, bounding box diagonal, path length, cumulative angle changes,
/// maximum speed, and total duration. It uses Weighted Euclidean Distance to match
/// an input gesture against a set of templates.
///
/// # Example (Happy Path)
///
/// ```rust
/// use quickdraw::gesture::rubine::RubineRecognizer;
/// use quickdraw::gesture::GestureRecognizer;
/// use quickdraw::types::GestureCapture;
///
/// let recognizer = RubineRecognizer::new();
/// let capture = GestureCapture {
///     points: vec![(0.0, 0.0), (10.0, 0.0), (20.0, 10.0)],
///     timestamps: vec![0, 100, 200],
/// };
///
/// // Create a template from a capture
/// let template = recognizer.create_template("flick".to_string(), &capture);
///
/// // Recognize a gesture against templates
/// let match_result = recognizer.recognize(&capture, &[template]);
/// assert!(match_result.is_some());
/// assert_eq!(match_result.unwrap().gesture_id, "flick");
/// ```
pub struct RubineRecognizer {}

impl RubineRecognizer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn extract_features(capture: &GestureCapture) -> [f64; 13] {
        let n = capture.points.len();
        if n < 3 || capture.timestamps.len() < n {
            return [0.0; 13];
        }

        let p = &capture.points;
        let t = &capture.timestamps;

        // f0, f1: Cosine and Sine of initial angle (between point 0 and point 2)
        let dx20 = p[2].0 - p[0].0;
        let dy20 = p[2].1 - p[0].1;
        let dist20 = (dx20 * dx20 + dy20 * dy20).sqrt();
        let (f0, f1) = if dist20 > 0.0 {
            (dx20 / dist20, dy20 / dist20)
        } else {
            (1.0, 0.0)
        };

        // f2, f3: Length and Angle of the bounding box diagonal
        let mut min_x = p[0].0;
        let mut max_x = p[0].0;
        let mut min_y = p[0].1;
        let mut max_y = p[0].1;
        for i in 1..n {
            if p[i].0 < min_x { min_x = p[i].0; }
            if p[i].0 > max_x { max_x = p[i].0; }
            if p[i].1 < min_y { min_y = p[i].1; }
            if p[i].1 > max_y { max_y = p[i].1; }
        }
        let bdx = max_x - min_x;
        let bdy = max_y - min_y;
        let f2 = (bdx * bdx + bdy * bdy).sqrt();
        let f3 = bdy.atan2(bdx);

        // f4, f5, f6: Distance, Cosine, and Sine of the angle between first and last points
        let dxl0 = p[n - 1].0 - p[0].0;
        let dyl0 = p[n - 1].1 - p[0].1;
        let f4 = (dxl0 * dxl0 + dyl0 * dyl0).sqrt();
        let (f5, f6) = if f4 > 0.0 {
            (dxl0 / f4, dyl0 / f4)
        } else {
            (1.0, 0.0)
        };

        // f7: Total stroke length
        // f8, f9, f10: Total angle traversed, Total absolute angle traversed, Sum of squared angle changes
        // f11: Maximum speed squared
        let mut f7 = 0.0;
        let mut f8 = 0.0;
        let mut f9 = 0.0;
        let mut f10 = 0.0;
        let mut f11 = 0.0;

        let mut prev_angle = 0.0;
        for i in 1..n {
            let dx = p[i].0 - p[i - 1].0;
            let dy = p[i].1 - p[i - 1].1;
            let dist_sq = dx * dx + dy * dy;
            let dist = dist_sq.sqrt();
            f7 += dist;

            let dt = t[i].saturating_sub(t[i - 1]) as f64;
            if dt > 0.0 {
                let speed_sq = dist_sq / (dt * dt);
                if speed_sq > f11 {
                    f11 = speed_sq;
                }
            }

            let angle = dy.atan2(dx);
            if i > 1 {
                let mut delta = angle - prev_angle;
                // Normalize delta to [-PI, PI]
                while delta > PI { delta -= 2.0 * PI; }
                while delta < -PI { delta += 2.0 * PI; }

                f8 += delta;
                f9 += delta.abs();
                f10 += delta * delta;
            }
            prev_angle = angle;
        }

        // f12: Total duration
        let f12 = (t[n - 1] - t[0]) as f64;

        [f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12]
    }
}

impl GestureRecognizer for RubineRecognizer {
    fn recognize(
        &self,
        capture: &GestureCapture,
        templates: &[GestureTemplate],
    ) -> Option<GestureMatch> {
        if capture.points.len() < 3 || templates.is_empty() {
            return None;
        }

        let input_features = Self::extract_features(capture);
        let mut best_distance = f64::MAX;
        let mut best_template = None;

        for template in templates {
            if template.algorithm != self.name() {
                continue;
            }

            if let Some(template_features) = &template.features {
                if template_features.len() != 13 {
                    continue;
                }

                let mut distance = 0.0;
                for i in 0..13 {
                    let diff = input_features[i] - template_features[i];
                    distance += RUBINE_WEIGHTS[i] * diff * diff;
                }

                if distance < best_distance {
                    best_distance = distance;
                    best_template = Some(&template.name);
                }
            }
        }

        let score = 1.0 / (1.0 + best_distance);

        best_template.map(|name| GestureMatch {
            gesture_id: name.clone(),
            confidence: score,
        })
    }

    fn create_template(&self, name: String, capture: &GestureCapture) -> GestureTemplate {
        let features = Self::extract_features(capture);

        GestureTemplate {
            name,
            template_points: capture.points.clone(),
            algorithm: self.name().to_string(),
            features: Some(features.to_vec()),
        }
    }

    fn name(&self) -> &str {
        "rubine"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_capture(points: Vec<(f64, f64)>, timestamps: Vec<u64>) -> GestureCapture {
        GestureCapture { points, timestamps }
    }

    #[test]
    fn test_extract_features_too_few_points() {
        let capture = dummy_capture(vec![(0.0, 0.0), (10.0, 10.0)], vec![0, 100]);
        let features = RubineRecognizer::extract_features(&capture);
        assert_eq!(features, [0.0; 13]);
    }

    #[test]
    fn test_extract_features_horizontal_line() {
        let capture = dummy_capture(
            vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)],
            vec![0, 100, 200]
        );
        let features = RubineRecognizer::extract_features(&capture);

        // f0: cos(initial angle) should be 1.0 (pointing right)
        assert!((features[0] - 1.0).abs() < 1e-6);
        // f1: sin(initial angle) should be 0.0
        assert!(features[1].abs() < 1e-6);
        // f7: total length should be 20.0
        assert!((features[7] - 20.0).abs() < 1e-6);
        // f12: total duration should be 200.0
        assert!((features[12] - 200.0).abs() < 1e-6);
    }

    #[test]
    fn test_recognize_exact_match() {
        let recognizer = RubineRecognizer::new();
        let capture = dummy_capture(
            vec![(0.0, 0.0), (5.0, 5.0), (10.0, 10.0), (15.0, 10.0), (20.0, 10.0)],
            vec![0, 50, 100, 150, 200]
        );
        let template = recognizer.create_template("test-gesture".to_string(), &capture);

        let match_result = recognizer.recognize(&capture, &[template]);
        assert!(match_result.is_some());
        let m = match_result.unwrap();
        assert_eq!(m.gesture_id, "test-gesture");
        // Distance should be 0.0, so confidence should be 1.0
        assert!((m.confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_recognize_slight_variation() {
        let recognizer = RubineRecognizer::new();
        let capture1 = dummy_capture(
            vec![(0.0, 0.0), (5.0, 5.0), (10.0, 10.0), (15.0, 10.0), (20.0, 10.0)],
            vec![0, 50, 100, 150, 200]
        );
        let template = recognizer.create_template("test-gesture".to_string(), &capture1);

        // Slight variation in points and timing
        let capture2 = dummy_capture(
            vec![(0.0, 0.0), (5.1, 4.9), (10.0, 10.1), (14.9, 10.0), (20.0, 10.0)],
            vec![0, 55, 105, 155, 210]
        );

        let match_result = recognizer.recognize(&capture2, &[template]);
        assert!(match_result.is_some());
        let m = match_result.unwrap();
        assert_eq!(m.gesture_id, "test-gesture");
        // Confidence should still be relatively high for slight variations
        assert!(m.confidence > 0.8, "Confidence was {}, expected > 0.8", m.confidence);
    }
}
