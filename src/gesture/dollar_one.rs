use std::f64::consts::PI;

use crate::types::{GestureCapture, GestureMatch, GestureTemplate};
use super::GestureRecognizer;

const N_POINTS: usize = 64;
const SQUARE_SIZE: f64 = 250.0;
const DIAGONAL: f64 = 353.5533905932738; // sqrt(250^2 + 250^2)
const HALF_DIAGONAL: f64 = 0.5 * DIAGONAL;

const ANGLE_RANGE: f64 = 15.0 * (PI / 180.0);
const ANGLE_PRECISION: f64 = 2.0 * (PI / 180.0);
const PHI: f64 = 0.6180339887498948; // Golden Ratio

pub struct DollarOneRecognizer {}

impl DollarOneRecognizer {
    pub fn new() -> Self {
        Self {}
    }

    fn normalize(&self, points: &[(f64, f64)]) -> Vec<(f64, f64)> {
        let mut pts = resample(points, N_POINTS);
        pts = scale_to(&pts, SQUARE_SIZE);
        pts = translate_to_origin(&pts);
        pts
    }
}

impl GestureRecognizer for DollarOneRecognizer {
    fn recognize(
        &self,
        capture: &GestureCapture,
        templates: &[GestureTemplate],
    ) -> Option<GestureMatch> {
        if capture.points.len() < 2 || templates.is_empty() {
            return None;
        }

        let points = self.normalize(&capture.points);

        let mut best_distance = f64::MAX;
        let mut best_template = None;

        for template in templates {
            if template.algorithm != self.name() {
                continue;
            }

            let distance = distance_at_best_angle(
                &points,
                &template.template_points,
                -ANGLE_RANGE,
                ANGLE_RANGE,
                ANGLE_PRECISION,
            );

            if distance < best_distance {
                best_distance = distance;
                best_template = Some(&template.name);
            }
        }

        let score = 1.0 - (best_distance / HALF_DIAGONAL);

        best_template.map(|name| GestureMatch {
            gesture_id: name.clone(),
            confidence: score,
        })
    }

    fn create_template(&self, name: String, capture: &GestureCapture) -> GestureTemplate {
        let template_points = self.normalize(&capture.points);

        GestureTemplate {
            name,
            template_points,
            algorithm: self.name().to_string(),
            features: None,
        }
    }

    fn name(&self) -> &str {
        "dollar_one"
    }
}

// --- Algorithm Steps ---

fn resample(points: &[(f64, f64)], n: usize) -> Vec<(f64, f64)> {
    if points.is_empty() {
        return vec![];
    }
    
    if points.len() == 1 {
        return vec![points[0]; n];
    }

    let mut resampled = vec![points[0]];
    let ideal_spacing = path_length(points) / ((n - 1) as f64);
    let mut current_distance = 0.0;
    
    let mut i = 1;
    let mut pts = points.to_vec();

    while i < pts.len() {
        let p1 = pts[i - 1];
        let p2 = pts[i];
        let d = distance(p1, p2);

        if current_distance + d >= ideal_spacing && ideal_spacing > 0.0 {
            let ratio = (ideal_spacing - current_distance) / d;
            let qx = p1.0 + ratio * (p2.0 - p1.0);
            let qy = p1.1 + ratio * (p2.1 - p1.1);
            let q = (qx, qy);

            resampled.push(q);
            pts.insert(i, q);
            current_distance = 0.0;
            i += 1; // Move past the newly inserted point
        } else {
            current_distance += d;
            i += 1;
        }
        
        if resampled.len() >= n {
            break;
        }
    }

    // Floating point errors might leave us short
    while resampled.len() < n {
        resampled.push(*pts.last().unwrap());
    }

    resampled.truncate(n);
    
    resampled
}

fn rotate_by(points: &[(f64, f64)], radians: f64) -> Vec<(f64, f64)> {
    let c = centroid(points);
    let cos = radians.cos();
    let sin = radians.sin();

    points
        .iter()
        .map(|&(x, y)| {
            let qx = (x - c.0) * cos - (y - c.1) * sin + c.0;
            let qy = (x - c.0) * sin + (y - c.1) * cos + c.1;
            (qx, qy)
        })
        .collect()
}

fn scale_to(points: &[(f64, f64)], size: f64) -> Vec<(f64, f64)> {
    let (min_x, max_x, min_y, max_y) = bounding_box(points);
    let width = max_x - min_x;
    let height = max_y - min_y;

    // Check if the gesture is essentially a 1D line (ratio <= 0.3)
    // Non-uniform scaling of a 1D line magnifies small wobble errors to the full square size.
    let is_1d = (width.min(height) / width.max(height).max(1.0)) <= 0.3;

    points
        .iter()
        .map(|&(x, y)| {
            if is_1d {
                let max_dim = width.max(height).max(1.0);
                let qx = (x - min_x) * (size / max_dim);
                let qy = (y - min_y) * (size / max_dim);
                (qx, qy)
            } else {
                let qx = if width == 0.0 { x } else { (x - min_x) * (size / width) };
                let qy = if height == 0.0 { y } else { (y - min_y) * (size / height) };
                (qx, qy)
            }
        })
        .collect()
}

fn translate_to_origin(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let c = centroid(points);
    points.iter().map(|&(x, y)| (x - c.0, y - c.1)).collect()
}

// --- Matching & Golden Section Search ---

fn distance_at_best_angle(
    points: &[(f64, f64)],
    template: &[(f64, f64)],
    theta_a: f64,
    theta_b: f64,
    theta_delta: f64,
) -> f64 {
    let mut a = theta_a;
    let mut b = theta_b;

    let mut x1 = PHI * a + (1.0 - PHI) * b;
    let mut f1 = distance_at_angle(points, template, x1);

    let mut x2 = (1.0 - PHI) * a + PHI * b;
    let mut f2 = distance_at_angle(points, template, x2);

    let mut iters = 0;
    let max_iters = 100; // prevent infinite loops if precision issues occur

    while (b - a).abs() > theta_delta && iters < max_iters {
        if f1 < f2 {
            b = x2;
            x2 = x1;
            f2 = f1;
            x1 = PHI * a + (1.0 - PHI) * b;
            f1 = distance_at_angle(points, template, x1);
        } else {
            a = x1;
            x1 = x2;
            f1 = f2;
            x2 = (1.0 - PHI) * a + PHI * b;
            f2 = distance_at_angle(points, template, x2);
        }
        iters += 1;
    }

    f1.min(f2)
}

fn distance_at_angle(points: &[(f64, f64)], template: &[(f64, f64)], radians: f64) -> f64 {
    let rotated = rotate_by(points, radians);
    path_distance(&rotated, template)
}

fn path_distance(pts1: &[(f64, f64)], pts2: &[(f64, f64)]) -> f64 {
    let len = pts1.len().min(pts2.len()) as f64;
    if len == 0.0 {
        return f64::MAX;
    }
    
    let sum: f64 = pts1
        .iter()
        .zip(pts2.iter())
        .map(|(&p1, &p2)| distance(p1, p2))
        .sum();
    sum / len
}

// --- Geometry Helpers ---

fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    (dx * dx + dy * dy).sqrt()
}

fn path_length(points: &[(f64, f64)]) -> f64 {
    let mut d = 0.0;
    for i in 1..points.len() {
        d += distance(points[i - 1], points[i]);
    }
    d
}

fn centroid(points: &[(f64, f64)]) -> (f64, f64) {
    if points.is_empty() {
        return (0.0, 0.0);
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    for &(x, y) in points {
        cx += x;
        cy += y;
    }
    let len = points.len() as f64;
    (cx / len, cy / len)
}

fn bounding_box(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for &(x, y) in points {
        if x < min_x { min_x = x; }
        if x > max_x { max_x = x; }
        if y < min_y { min_y = y; }
        if y > max_y { max_y = y; }
    }

    (min_x, max_x, min_y, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_capture(points: Vec<(f64, f64)>) -> GestureCapture {
        let timestamps = vec![0; points.len()];
        GestureCapture { points, timestamps }
    }

    #[test]
    fn test_resample_count() {
        let points = vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)];
        let resampled = resample(&points, 64);
        assert_eq!(resampled.len(), 64);
    }

    #[test]
    fn test_recognize_exact_match() {
        let recognizer = DollarOneRecognizer::new();
        
        let shape = vec![
            (0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0),
            (30.0, 10.0), (30.0, 20.0), (30.0, 30.0)
        ];
        
        let capture = dummy_capture(shape.clone());
        let template = recognizer.create_template("L-shape".to_string(), &capture);
        
        let match_result = recognizer.recognize(&capture, &[template]);
        assert!(match_result.is_some());
        
        let m = match_result.unwrap();
        assert_eq!(m.gesture_id, "L-shape");
        assert!(m.confidence > 0.95);
    }

    #[test]
    fn test_recognize_scaled() {
        let recognizer = DollarOneRecognizer::new();
        
        // Base L-shape
        let base_shape = vec![
            (0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0),
            (30.0, 10.0), (30.0, 20.0), (30.0, 30.0)
        ];
        let base_capture = dummy_capture(base_shape.clone());
        let template = recognizer.create_template("L-shape".to_string(), &base_capture);
        
        // Scaled L-shape (no big rotation, just wobble)
        let mut variant_shape = Vec::new();
        for &(x, y) in &base_shape {
            // Scale by 2.0, translate by 50, rotate by 5 deg
            let rad = 5.0 * (PI / 180.0);
            let sx = x * 2.0;
            let sy = y * 2.0;
            let rx = sx * rad.cos() - sy * rad.sin() + 50.0;
            let ry = sx * rad.sin() + sy * rad.cos() + 50.0;
            variant_shape.push((rx, ry));
        }
        let variant_capture = dummy_capture(variant_shape);
        
        let match_result = recognizer.recognize(&variant_capture, &[template]);
        
        if let Some(m) = &match_result {
            println!("Scaled match confidence: {}", m.confidence);
        } else {
            let pts1 = recognizer.normalize(&variant_capture.points);
            println!("Pts1: {:?}", &pts1[0..5]);
        }
        
        assert!(match_result.is_some(), "Should match scaled version with wobble");
        
        let m = match_result.unwrap();
        assert_eq!(m.gesture_id, "L-shape");
        assert!(m.confidence > 0.85);
    }

    #[test]
    fn test_reject_different_gesture() {
        let recognizer = DollarOneRecognizer::new();
        
        // L-shape template
        let shape1 = vec![
            (0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0),
            (30.0, 10.0), (30.0, 20.0), (30.0, 30.0)
        ];
        let cap1 = dummy_capture(shape1);
        let template = recognizer.create_template("L-shape".to_string(), &cap1);
        
        // A completely different shape: a circle or square
        let mut shape2 = Vec::new();
        for i in 0..30 {
            let t = i as f64 * (2.0 * PI / 30.0);
            shape2.push((50.0 + 30.0 * t.cos(), 50.0 + 30.0 * t.sin()));
        }
        let cap2 = dummy_capture(shape2);
        
        let match_result = recognizer.recognize(&cap2, &[template]);
        
        if let Some(m) = &match_result {
            println!("Different gesture matched with confidence: {}", m.confidence);
            // Since we removed threshold, it SHOULD match but with low confidence
            assert!(m.confidence < 0.8);
        }
    }

    #[test]
    fn test_direction_discrimination() {
        let recognizer = DollarOneRecognizer::new();
        
        // L-shape (down then right) -> Actually the one above is right then down.
        // Let's do Down-Right
        let shape1 = vec![
            (0.0, 0.0), (0.0, 10.0), (0.0, 20.0), (0.0, 30.0),
            (10.0, 30.0), (20.0, 30.0), (30.0, 30.0)
        ];
        let cap1 = dummy_capture(shape1);
        let template = recognizer.create_template("L-shape-down-right".to_string(), &cap1);
        
        // Mirrored L-shape (down then left)
        let shape2 = vec![
            (0.0, 0.0), (0.0, 10.0), (0.0, 20.0), (0.0, 30.0),
            (-10.0, 30.0), (-20.0, 30.0), (-30.0, 30.0)
        ];
        let cap2 = dummy_capture(shape2);
        
        let match_result = recognizer.recognize(&cap2, &[template]);
        
        if let Some(m) = &match_result {
            println!("Mirrored gesture matched with confidence: {}", m.confidence);
            let pts1 = recognizer.normalize(&cap1.points);
            let pts2 = recognizer.normalize(&cap2.points);
            println!("Pts1: {:?}", &pts1[0..5]);
            println!("Pts2: {:?}", &pts2[0..5]);
            
            // Should match with low confidence due to direction difference
            assert!(m.confidence < 0.8);
        }
    }
}
