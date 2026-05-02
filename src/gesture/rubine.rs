use crate::types::{GestureCapture, GestureMatch, GestureTemplate};
use super::GestureRecognizer;

/// Rubine Gesture Recognizer (1991)
///
/// This implementation uses the 13 dynamic features described in the paper:
/// - f0, f1: Cosine and sine of initial angle
/// - f2, f3: Length and angle of bounding box diagonal
/// - f4, f5, f6: Distance, cosine, and sine of angle between first and last points
/// - f7: Total stroke length
/// - f8, f9, f10: Total angle, total absolute angle, sum of squared angle changes
/// - f11: Maximum speed squared
/// - f12: Total duration
///
/// Matching is performed using a normalized Euclidean distance on these features.
///
/// # Example
///
/// ```rust
/// use quickdraw::gesture::rubine::RubineRecognizer;
/// use quickdraw::gesture::GestureRecognizer;
/// use quickdraw::types::GestureCapture;
///
/// let recognizer = RubineRecognizer::new();
/// let capture = GestureCapture {
///     points: vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)],
///     timestamps: vec![0, 10, 20],
/// };
/// let template = recognizer.create_template("line".to_string(), &capture);
/// let match_result = recognizer.recognize(&capture, &[template]).unwrap();
/// assert_eq!(match_result.gesture_id, "line");
/// assert!(match_result.confidence > 0.9);
/// ```
pub struct RubineRecognizer {}

impl RubineRecognizer {
    pub fn new() -> Self {
        Self {}
    }

    fn extract_features(&self, capture: &GestureCapture) -> Vec<f64> {
        let n = capture.points.len();
        if n < 3 || capture.timestamps.len() < n {
            return vec![0.0; 13];
        }

        let p = &capture.points;
        let t = &capture.timestamps;

        let dx0 = p[1].0 - p[0].0;
        let dy0 = p[1].1 - p[0].1;
        let d01 = (dx0 * dx0 + dy0 * dy0).sqrt().max(1.0);

        // f0, f1: initial angle (vector p0 to p1)
        let f0 = dx0 / d01;
        let f1 = dy0 / d01;

        // f2, f3: bounding box diagonal
        let mut min_x = p[0].0;
        let mut max_x = p[0].0;
        let mut min_y = p[0].1;
        let mut max_y = p[0].1;
        for &(x, y) in p {
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
        }
        let bb_dx = max_x - min_x;
        let bb_dy = max_y - min_y;
        let f2 = (bb_dx * bb_dx + bb_dy * bb_dy).sqrt();
        let f3 = bb_dy.atan2(bb_dx);

        // f4, f5, f6: start to end
        let d_last_x = p[n - 1].0 - p[0].0;
        let d_last_y = p[n - 1].1 - p[0].1;
        let f4 = (d_last_x * d_last_x + d_last_y * d_last_y).sqrt();
        let f5 = if f4 > 0.0 { d_last_x / f4 } else { 0.0 };
        let f6 = if f4 > 0.0 { d_last_y / f4 } else { 0.0 };

        // f7: total length
        let mut f7 = 0.0;
        for i in 0..n - 1 {
            let dx = p[i + 1].0 - p[i].0;
            let dy = p[i + 1].1 - p[i].1;
            f7 += (dx * dx + dy * dy).sqrt();
        }

        // f8, f9, f10: angles
        let mut f8 = 0.0;
        let mut f9 = 0.0;
        let mut f10 = 0.0;

        let mut prev_theta = dy0.atan2(dx0);
        for i in 1..n - 1 {
            let dx = p[i + 1].0 - p[i].0;
            let dy = p[i + 1].1 - p[i].1;
            if dx == 0.0 && dy == 0.0 {
                continue;
            }
            let theta = dy.atan2(dx);
            let mut d_theta = theta - prev_theta;
            // Normalize d_theta to [-PI, PI]
            while d_theta > std::f64::consts::PI {
                d_theta -= 2.0 * std::f64::consts::PI;
            }
            while d_theta < -std::f64::consts::PI {
                d_theta += 2.0 * std::f64::consts::PI;
            }

            f8 += d_theta;
            f9 += d_theta.abs();
            f10 += d_theta * d_theta;
            prev_theta = theta;
        }

        // f11: max speed squared
        let mut f11 = 0.0;
        for i in 0..n - 1 {
            let dx = p[i + 1].0 - p[i].0;
            let dy = p[i + 1].1 - p[i].1;
            let dt = t[i + 1].saturating_sub(t[i]) as f64;
            if dt > 0.0 {
                let speed_sq = (dx * dx + dy * dy) / (dt * dt);
                if speed_sq > f11 {
                    f11 = speed_sq;
                }
            }
        }

        // f12: total duration
        let f12 = t[n - 1].saturating_sub(t[0]) as f64;

        vec![f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12]
    }
}

impl GestureRecognizer for RubineRecognizer {
    fn recognize(
        &self,
        capture: &GestureCapture,
        templates: &[GestureTemplate],
    ) -> Option<GestureMatch> {
        if templates.is_empty() {
            return None;
        }

        let input_features = self.extract_features(capture);
        if input_features.iter().all(|&f| f == 0.0) {
            return None;
        }

        let mut best_confidence = -1.0;
        let mut best_name = None;

        for template in templates {
            if template.algorithm != self.name() {
                continue;
            }

            let template_features = if let Some(ref f) = template.features {
                f.clone()
            } else {
                continue;
            };

            if template_features.len() != 13 {
                continue;
            }

            let mut dist_sq = 0.0;
            for i in 0..13 {
                let is_angle = matches!(i, 0 | 1 | 3 | 5 | 6 | 8 | 9 | 10);
                let diff = if is_angle {
                    input_features[i] - template_features[i]
                } else {
                    (input_features[i] - template_features[i]) / template_features[i].max(1.0)
                };

                // Tuning weights
                let weight = match i {
                    11 => 0.1, // Max speed sq can be very volatile
                    12 => 0.5, // Duration can be somewhat volatile
                    _ => 1.0,
                };

                dist_sq += (diff * weight).powi(2);
            }

            let dist = dist_sq.sqrt();
            // Map distance to confidence.
            // Heuristic: allow for ~15% human variance. 13 features * (0.15^2) is ~0.3. Sqrt is ~0.55.
            let confidence = (1.0 - (dist / 2.0)).max(0.0);

            if confidence > best_confidence {
                best_confidence = confidence;
                best_name = Some(template.name.clone());
            }
        }

        best_name.map(|name| GestureMatch {
            gesture_id: name,
            confidence: best_confidence,
        })
    }

    fn create_template(&self, name: String, capture: &GestureCapture) -> GestureTemplate {
        let features = self.extract_features(capture);

        GestureTemplate {
            name,
            template_points: capture.points.clone(),
            algorithm: self.name().to_string(),
            features: Some(features),
        }
    }

    fn name(&self) -> &str {
        "rubine"
    }
}
