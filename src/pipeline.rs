use std::collections::HashMap;
use std::time::Instant;
use anyhow::{anyhow, Result};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

use crate::config::{Config, TriggerConfig, GestureConfig};
use crate::types::{GestureCapture, InputEvent, InputEventType, MouseButton};
use crate::input::{InputSource, raw_input::RawInputSource, hook::HookInputSource};
use crate::gesture::{GestureRecognizer, dollar_one::DollarOneRecognizer};
use crate::types::GestureTemplate;
use crate::output::{OutputAction, create_action};
use crate::audio::AudioPlayer;
use crate::ui::trace::{TraceOverlay, TraceCommand};

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerState {
    Idle,
    WaitingForSecond { first: String },
    GestureActive { origin: (f64, f64) },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerSignal {
    Pass(InputEvent),
    GestureStarted,
    GesturePoint(f64, f64),
    GestureComplete,
    Nothing,
}

pub struct CaptureRequest {
    pub result_tx: oneshot::Sender<CaptureResult>,
    pub cancel_rx: oneshot::Receiver<()>,
}

pub struct CaptureResult {
    pub raw: GestureCapture,
    pub template: GestureTemplate,
}

impl InputEvent {
    fn matches_key(&self, key_name: &str) -> Option<bool> {
        match &self.event_type {
            InputEventType::MouseButton { button, pressed } => {
                let matches = match button {
                    MouseButton::Left => key_name.eq_ignore_ascii_case("Mouse1") || key_name.eq_ignore_ascii_case("Left"),
                    MouseButton::Right => key_name.eq_ignore_ascii_case("Mouse2") || key_name.eq_ignore_ascii_case("Right"),
                    MouseButton::Middle => key_name.eq_ignore_ascii_case("Mouse3") || key_name.eq_ignore_ascii_case("Middle"),
                    MouseButton::X1 => key_name.eq_ignore_ascii_case("Mouse4") || key_name.eq_ignore_ascii_case("X1"),
                    MouseButton::X2 => key_name.eq_ignore_ascii_case("Mouse5") || key_name.eq_ignore_ascii_case("X2"),
                };
                if matches {
                    return Some(*pressed);
                }
            }
            InputEventType::KeyboardKey { key, pressed } => {
                if key.0.eq_ignore_ascii_case(key_name) {
                    return Some(*pressed);
                }
            }
            _ => {}
        }
        None
    }
}

pub struct TriggerDetector {
    pub state: TriggerState,
    config: TriggerConfig,
}

impl TriggerDetector {
    pub fn new(config: TriggerConfig) -> Self {
        Self {
            state: TriggerState::Idle,
            config,
        }
    }

    fn get_cursor_pos() -> (f64, f64) {
        #[cfg(windows)]
        unsafe {
            let mut pos = windows::Win32::Foundation::POINT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pos);
            (pos.x as f64, pos.y as f64)
        }
        #[cfg(not(windows))]
        (0.0, 0.0)
    }

    pub fn process(&mut self, event: &InputEvent) -> TriggerSignal {
        match &self.config {
            TriggerConfig::Combo { key1, key2 } => {
                match &self.state {
                    TriggerState::Idle => {
                        if let Some(true) = event.matches_key(key1) {
                            self.state = TriggerState::WaitingForSecond { first: key1.clone() };
                            return TriggerSignal::Pass(event.clone());
                        }
                        if let Some(true) = event.matches_key(key2) {
                            self.state = TriggerState::WaitingForSecond { first: key2.clone() };
                            return TriggerSignal::Pass(event.clone());
                        }
                        TriggerSignal::Pass(event.clone())
                    }
                    TriggerState::WaitingForSecond { first: waiting_first } => {
                        let target_second = if waiting_first.eq_ignore_ascii_case(key1) { key2 } else { key1 };
                        
                        if let Some(true) = event.matches_key(target_second) {
                            let origin = Self::get_cursor_pos();
                            self.state = TriggerState::GestureActive { origin };
                            return TriggerSignal::GestureStarted;
                        } else if let Some(false) = event.matches_key(waiting_first) {
                            self.state = TriggerState::Idle;
                            return TriggerSignal::Pass(event.clone());
                        }
                        TriggerSignal::Pass(event.clone())
                    }
                    TriggerState::GestureActive { origin: _ } => {
                        match &event.event_type {
                            InputEventType::MouseMove { dx, dy } => {
                                TriggerSignal::GesturePoint(*dx as f64, *dy as f64)
                            }
                            _ => {
                                if let Some(false) = event.matches_key(key1) {
                                    self.state = TriggerState::Idle;
                                    return TriggerSignal::GestureComplete;
                                }
                                if let Some(false) = event.matches_key(key2) {
                                    self.state = TriggerState::Idle;
                                    return TriggerSignal::GestureComplete;
                                }
                                TriggerSignal::Nothing
                            }
                        }
                    }
                }
            }
            TriggerConfig::Single { key1: target_key } => {
                match &self.state {
                    TriggerState::Idle => {
                        if let Some(true) = event.matches_key(target_key) {
                            let origin = Self::get_cursor_pos();
                            self.state = TriggerState::GestureActive { origin };
                            return TriggerSignal::GestureStarted;
                        }
                        TriggerSignal::Pass(event.clone())
                    }
                    TriggerState::GestureActive { origin: _ } => {
                        match &event.event_type {
                            InputEventType::MouseMove { dx, dy } => {
                                TriggerSignal::GesturePoint(*dx as f64, *dy as f64)
                            }
                            _ => {
                                if let Some(false) = event.matches_key(target_key) {
                                    self.state = TriggerState::Idle;
                                    return TriggerSignal::GestureComplete;
                                }
                                TriggerSignal::Nothing
                            }
                        }
                    }
                    _ => TriggerSignal::Pass(event.clone()),
                }
            }
        }
    }
}

pub struct GestureAccumulator {
    capture_points: Vec<(f64, f64)>,
    capture_timestamps: Vec<u64>,
    current_x: f64,
    current_y: f64,
    start_time: Instant,
    origin_pos: (f64, f64),
}

impl GestureAccumulator {
    fn new() -> Self {
        Self {
            capture_points: Vec::new(),
            capture_timestamps: Vec::new(),
            current_x: 0.0,
            current_y: 0.0,
            start_time: Instant::now(),
            origin_pos: (0.0, 0.0),
        }
    }
    
    fn start(&mut self, origin: (f64, f64)) {
        self.capture_points.clear();
        self.capture_timestamps.clear();
        self.current_x = 0.0;
        self.current_y = 0.0;
        self.start_time = Instant::now();
        self.origin_pos = origin;
    }
    
    fn add_point(&mut self, dx: f64, dy: f64) {
        self.current_x += dx;
        self.current_y += dy;
        self.capture_points.push((self.current_x, self.current_y));
        self.capture_timestamps.push(self.start_time.elapsed().as_millis() as u64);
    }
    
    fn finish(&mut self) -> GestureCapture {
        let capture = GestureCapture {
            points: self.capture_points.clone(),
            timestamps: self.capture_timestamps.clone(),
        };
        self.capture_points.clear();
        self.capture_timestamps.clear();
        capture
    }
}

fn compute_path_length(capture: &GestureCapture) -> f64 {
    let mut length = 0.0;
    for i in 1..capture.points.len() {
        let (x1, y1) = capture.points[i - 1];
        let (x2, y2) = capture.points[i];
        length += ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
    }
    length
}

fn compute_speed(capture: &GestureCapture) -> f64 {
    let length = compute_path_length(capture);
    let duration = capture.timestamps.last().copied().unwrap_or(0);
    if duration == 0 {
        0.0
    } else {
        length / duration as f64
    }
}

pub struct Pipeline {
    mouse_input_source: Box<dyn InputSource>,
    keyboard_input_source: Box<dyn InputSource>,
    recognizer: Box<dyn GestureRecognizer>,
    templates: Vec<GestureTemplate>,
    actions: HashMap<String, Box<dyn OutputAction>>,
    gesture_configs: HashMap<String, GestureConfig>,
    trigger: TriggerDetector,
    audio: AudioPlayer,
    config: Config,
    capture_request_rx: mpsc::Receiver<CaptureRequest>,
    trace_overlay: Option<TraceOverlay>,
}

pub fn build_pipeline(config: Config, capture_request_rx: mpsc::Receiver<CaptureRequest>) -> Result<Pipeline> {
    let mouse_input_source: Box<dyn InputSource> = match config.general.mouse_input_method.as_str() {
        "raw_input" => Box::new(RawInputSource::new(true, false)),
        "hook" => Box::new(HookInputSource::new()),
        other => return Err(anyhow!("Unknown mouse input method: {}", other)),
    };

    let keyboard_input_source: Box<dyn InputSource> = match config.general.keyboard_input_method.as_str() {
        "raw_input" => Box::new(RawInputSource::new(false, true)),
        "hook" => Box::new(HookInputSource::new()),
        other => return Err(anyhow!("Unknown keyboard input method: {}", other)),
    };

    let recognizer: Box<dyn GestureRecognizer> = match config.general.recognizer.as_str() {
        "dollar_one" => Box::new(DollarOneRecognizer::new()),
        other => return Err(anyhow!("Unknown recognizer: {}", other)),
    };

    let profile = crate::config::load_gesture_profile(&config.general.gesture_profile)?;

    let mut templates = Vec::new();
    let mut actions = HashMap::new();
    let mut gesture_configs = HashMap::new();

    for gesture in profile.gestures {
        let name = gesture.name.clone();
        let template_points = gesture.pattern.template_points.iter().map(|p| (p[0], p[1])).collect();

        // Every template is added to the recognizer's pool
        templates.push(GestureTemplate {
            name: name.clone(),
            template_points,
            algorithm: gesture.pattern.algorithm.clone(),
        });

        // Last occurrence wins for actions and config overrides
        let action = create_action(&gesture.action)?;
        actions.insert(name.clone(), action);
        gesture_configs.insert(name, gesture);
    }

    let trigger = TriggerDetector::new(config.trigger.clone());
    let audio = AudioPlayer::new(config.audio.clone());

    let trace_overlay = if config.general.trace_overlay_enabled {
        Some(TraceOverlay::new(config.general.clone()))
    } else {
        None
    };

    Ok(Pipeline {
        mouse_input_source,
        keyboard_input_source,
        recognizer,
        templates,
        actions,
        gesture_configs,
        trigger,
        audio,
        config,
        capture_request_rx,
        trace_overlay,
    })
}

impl Pipeline {
    pub async fn run(mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(256);
        self.mouse_input_source.start(tx.clone())?;
        self.keyboard_input_source.start(tx)?;

        let mut accumulator = GestureAccumulator::new();
        let mut active_capture_request: Option<(oneshot::Sender<CaptureResult>, oneshot::Receiver<()>)> = None;

        loop {
            tokio::select! {
                Some(req) = self.capture_request_rx.recv() => {
                    tracing::info!("Received capture request, entering capture mode");
                    active_capture_request = Some((req.result_tx, req.cancel_rx));
                }
                Some(event) = rx.recv() => {
                    let signal = self.trigger.process(&event);

                    match signal {
                        TriggerSignal::Pass(_) => {}
                        TriggerSignal::GestureStarted => {
                            let mut origin = (0.0, 0.0);
                            if let TriggerState::GestureActive { origin: o } = self.trigger.state {
                                origin = o;
                            }
                            accumulator.start(origin);
                            if let Some(overlay) = &self.trace_overlay {
                                overlay.send(TraceCommand::Begin(origin.0, origin.1));
                            }
                        }
                        TriggerSignal::GesturePoint(dx, dy) => {
                            accumulator.add_point(dx, dy);
                            if let Some(overlay) = &self.trace_overlay {
                                overlay.send(TraceCommand::AddPoint(accumulator.origin_pos.0 + accumulator.current_x, accumulator.origin_pos.1 + accumulator.current_y));
                            }
                        }
                        TriggerSignal::GestureComplete => {
                            let origin_pos = accumulator.origin_pos;
                            let capture = accumulator.finish();
                            if let Some(overlay) = &self.trace_overlay {
                                overlay.send(TraceCommand::End);
                            }

                            if let Some((result_tx, mut cancel_rx)) = active_capture_request.take() {
                                // Check if capture was cancelled
                                if cancel_rx.try_recv().is_ok() {
                                    tracing::info!("Capture was cancelled, discarding result");
                                } else {
                                    // We are in capture mode. Create template and send back result.
                                    tracing::info!("Capture completed, sending result to UI");
                                    let template = self.recognizer.create_template("".to_string(), &capture);
                                    let result = CaptureResult {
                                        raw: capture,
                                        template,
                                    };
                                    let _ = result_tx.send(result);
                                }
                            } else {
                                // Normal recognition mode.
                                if let Some(match_result) = self.recognizer.recognize(&capture, &self.templates) {
                                    let gesture_id = &match_result.gesture_id;
                                    let confidence = match_result.confidence;

                                    let threshold = self.gesture_configs.get(gesture_id)
                                        .and_then(|g| g.confidence_threshold)
                                        .unwrap_or(self.config.general.confidence_threshold);

                                    if confidence >= threshold {
                                        let path_length = compute_path_length(&capture);
                                        let speed = compute_speed(&capture);

                                        let mut constraints_ok = true;
                                        if let Some(config) = self.gesture_configs.get(gesture_id) {
                                            if let Some(min_len) = config.min_path_length_px {
                                                if path_length < min_len {
                                                    tracing::warn!("Gesture '{}' length {:.1}px below min {:.1}px", gesture_id, path_length, min_len);
                                                    constraints_ok = false;
                                                }
                                            }
                                            if let Some(max_len) = config.max_path_length_px {
                                                if path_length > max_len {
                                                    tracing::warn!("Gesture '{}' length {:.1}px above max {:.1}px", gesture_id, path_length, max_len);
                                                    constraints_ok = false;
                                                }
                                            }
                                            if let Some(min_speed) = config.min_speed_px_per_ms {
                                                if speed < min_speed {
                                                    tracing::warn!("Gesture '{}' speed {:.2}px/ms below min {:.2}px/ms", gesture_id, speed, min_speed);
                                                    constraints_ok = false;
                                                }
                                            }
                                            if let Some(max_speed) = config.max_speed_px_per_ms {
                                                if speed > max_speed {
                                                    tracing::warn!("Gesture '{}' speed {:.2}px/ms above max {:.2}px/ms", gesture_id, speed, max_speed);
                                                    constraints_ok = false;
                                                }
                                            }
                                        }

                                        if constraints_ok {
                                            tracing::info!("Gesture matched: {} (confidence: {:.2})", gesture_id, confidence);

                                            let sound_override = self.gesture_configs.get(gesture_id)
                                                .and_then(|g| g.sound.as_deref());
                                            self.audio.play_success(sound_override);

                                            if let Some(action) = self.actions.get(gesture_id) {
                                                if let Err(e) = action.execute() {
                                                    tracing::error!("Action execution failed: {}", e);
                                                }
                                            }
                                        } else {
                                            self.audio.play_error();
                                        }
                                    } else {
                                        tracing::warn!("Gesture '{}' matched at {:.2} but below threshold {:.2}, ignoring", gesture_id, confidence, threshold);
                                        self.audio.play_error();
                                    }
                                } else {
                                    tracing::warn!("No gesture matched");
                                    self.audio.play_error();
                                }
                            }

                            if self.config.general.cursor_reset {
                                #[cfg(windows)]
                                unsafe {
                                    let _ = SetCursorPos(origin_pos.0 as i32, origin_pos.1 as i32);
                                }
                            }
                        }
                        TriggerSignal::Nothing => {}
                    }
                }
                else => break,
            }
        }

        Ok(())
    }

    pub async fn capture_one(mut self, name: String, action_str: String) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(256);
        self.mouse_input_source.start(tx.clone())?;
        self.keyboard_input_source.start(tx)?;

        println!("Hold your trigger key(s) and draw your gesture, then release...");

        let mut accumulator = GestureAccumulator::new();

        while let Some(event) = rx.recv().await {
            let signal = self.trigger.process(&event);

            match signal {
                TriggerSignal::Pass(_) => {}
                TriggerSignal::GestureStarted => {
                    let mut origin = (0.0, 0.0);
                    if let TriggerState::GestureActive { origin: o } = self.trigger.state {
                        origin = o;
                    }
                    accumulator.start(origin);
                    if let Some(overlay) = &self.trace_overlay {
                        overlay.send(TraceCommand::Begin(origin.0, origin.1));
                    }
                }
                TriggerSignal::GesturePoint(dx, dy) => {
                    accumulator.add_point(dx, dy);
                    if let Some(overlay) = &self.trace_overlay {
                        overlay.send(TraceCommand::AddPoint(accumulator.origin_pos.0 + accumulator.current_x, accumulator.origin_pos.1 + accumulator.current_y));
                    }
                }
                TriggerSignal::GestureComplete => {
                    let capture = accumulator.finish();
                    if let Some(overlay) = &self.trace_overlay {
                        overlay.send(TraceCommand::End);
                    }

                    let template = self.recognizer.create_template(name.clone(), &capture);
                    let action = crate::config::parse_action_str(&action_str)?;
                    
                    let template_points = template.template_points.iter().map(|p| [p.0, p.1]).collect();
                    let pattern = crate::config::GesturePatternConfig {
                        algorithm: template.algorithm,
                        template_points,
                    };

                    let gesture_config = crate::config::GestureConfig {
                        name: name.clone(),
                        action,
                        sound: None,
                        pattern,
                        raw: capture,
                        confidence_threshold: None,
                        min_speed_px_per_ms: None,
                        max_speed_px_per_ms: None,
                        min_path_length_px: None,
                        max_path_length_px: None,
                    };

                    let mut profile = crate::config::load_gesture_profile(&self.config.general.gesture_profile)?;
                    
                    if let Some(existing) = profile.gestures.iter_mut().find(|g| g.name == name) {
                        *existing = gesture_config;
                    } else {
                        profile.gestures.push(gesture_config);
                    }

                    crate::config::save_gesture_profile(&self.config.general.gesture_profile, &profile)?;
                    println!("Saved gesture '{}' -> {}", name, action_str);
                    
                    break;
                }
                TriggerSignal::Nothing => {}
            }
        }
        
        let _ = self.mouse_input_source.stop();
        let _ = self.keyboard_input_source.stop();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_state_machine() {
        let config = TriggerConfig::Combo {
            key1: "Mouse1".to_string(),
            key2: "Mouse2".to_string(),
        };
        let mut detector = TriggerDetector::new(config);

        let m1_down = InputEvent {
            event_type: InputEventType::MouseButton { button: MouseButton::Left, pressed: true },
            timestamp: 0,
        };
        let signal = detector.process(&m1_down);
        assert!(matches!(signal, TriggerSignal::Pass(_)));
        assert!(matches!(detector.state, TriggerState::WaitingForSecond { ref first } if first == "Mouse1"));

        let m2_down = InputEvent {
            event_type: InputEventType::MouseButton { button: MouseButton::Right, pressed: true },
            timestamp: 1,
        };
        let signal = detector.process(&m2_down);
        assert!(matches!(signal, TriggerSignal::GestureStarted));
        assert!(matches!(detector.state, TriggerState::GestureActive { .. }));

        let mouse_move = InputEvent {
            event_type: InputEventType::MouseMove { dx: 10, dy: -5 },
            timestamp: 2,
        };
        let signal = detector.process(&mouse_move);
        assert_eq!(signal, TriggerSignal::GesturePoint(10.0, -5.0));

        let m1_up = InputEvent {
            event_type: InputEventType::MouseButton { button: MouseButton::Left, pressed: false },
            timestamp: 3,
        };
        let signal = detector.process(&m1_up);
        assert_eq!(signal, TriggerSignal::GestureComplete);
        assert_eq!(detector.state, TriggerState::Idle);
    }
    
    #[test]
    fn test_trigger_abort_first_button() {
        let config = TriggerConfig::Combo {
            key1: "Mouse1".to_string(),
            key2: "Mouse2".to_string(),
        };
        let mut detector = TriggerDetector::new(config);

        let m1_down = InputEvent {
            event_type: InputEventType::MouseButton { button: MouseButton::Left, pressed: true },
            timestamp: 0,
        };
        detector.process(&m1_down);
        
        let m1_up = InputEvent {
            event_type: InputEventType::MouseButton { button: MouseButton::Left, pressed: false },
            timestamp: 1,
        };
        let signal = detector.process(&m1_up);
        assert!(matches!(signal, TriggerSignal::Pass(_)));
        assert_eq!(detector.state, TriggerState::Idle);
    }

    #[test]
    fn test_compute_path_length() {
        let capture = GestureCapture {
            points: vec![(0.0, 0.0), (3.0, 4.0), (3.0, 0.0)],
            timestamps: vec![0, 10, 20],
        };
        // distance((0,0), (3,4)) = 5
        // distance((3,4), (3,0)) = 4
        // total = 9
        assert_eq!(compute_path_length(&capture), 9.0);

        let empty_capture = GestureCapture {
            points: vec![],
            timestamps: vec![],
        };
        assert_eq!(compute_path_length(&empty_capture), 0.0);

        let single_point = GestureCapture {
            points: vec![(10.0, 10.0)],
            timestamps: vec![100],
        };
        assert_eq!(compute_path_length(&single_point), 0.0);
    }

    #[test]
    fn test_compute_speed() {
        let capture = GestureCapture {
            points: vec![(0.0, 0.0), (10.0, 0.0)],
            timestamps: vec![0, 100],
        };
        // length = 10, duration = 100
        // speed = 10 / 100 = 0.1
        assert_eq!(compute_speed(&capture), 0.1);

        let zero_duration = GestureCapture {
            points: vec![(0.0, 0.0), (10.0, 0.0)],
            timestamps: vec![0, 0],
        };
        assert_eq!(compute_speed(&zero_duration), 0.0);
    }
}
