use std::collections::HashMap;
use std::time::Instant;
use anyhow::{anyhow, Result};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

use crate::config::{Config, TriggerConfig, GestureConfig};
use crate::types::{GestureCapture, InputEvent, InputEventType, MouseButton};
use crate::input::{InputSource, raw_input::RawInputSource};
use crate::gesture::{GestureRecognizer, dollar_one::DollarOneRecognizer};
use crate::types::GestureTemplate;
use crate::output::{OutputAction, create_action};
use crate::audio::AudioPlayer;

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
        unsafe {
            let mut pos = windows::Win32::Foundation::POINT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pos);
            (pos.x as f64, pos.y as f64)
        }
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

pub struct Pipeline {
    input_source: Box<dyn InputSource>,
    recognizer: Box<dyn GestureRecognizer>,
    templates: Vec<GestureTemplate>,
    actions: HashMap<String, Box<dyn OutputAction>>,
    gesture_configs: HashMap<String, GestureConfig>,
    trigger: TriggerDetector,
    audio: AudioPlayer,
    config: Config,
    capture_request_rx: mpsc::Receiver<CaptureRequest>,
}

pub fn build_pipeline(config: Config, capture_request_rx: mpsc::Receiver<CaptureRequest>) -> Result<Pipeline> {
    let input_source: Box<dyn InputSource> = match config.general.input_method.as_str() {
        "raw_input" => Box::new(RawInputSource::new()),
        other => return Err(anyhow!("Unknown input method: {}", other)),
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

    Ok(Pipeline {
        input_source,
        recognizer,
        templates,
        actions,
        gesture_configs,
        trigger,
        audio,
        config,
        capture_request_rx,
    })
}

impl Pipeline {
    pub async fn run(mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(256);
        self.input_source.start(tx)?;

        let mut accumulator = GestureAccumulator::new();
        let mut active_capture_request: Option<oneshot::Sender<CaptureResult>> = None;

        loop {
            tokio::select! {
                Some(req) = self.capture_request_rx.recv() => {
                    tracing::info!("Received capture request, entering capture mode");
                    active_capture_request = Some(req.result_tx);
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
                        }
                        TriggerSignal::GesturePoint(dx, dy) => {
                            accumulator.add_point(dx, dy);
                        }
                        TriggerSignal::GestureComplete => {
                            let origin_pos = accumulator.origin_pos;
                            let capture = accumulator.finish();

                            if let Some(result_tx) = active_capture_request.take() {
                                // We are in capture mode. Create template and send back result.
                                tracing::info!("Capture completed, sending result to UI");
                                let template = self.recognizer.create_template("".to_string(), &capture);
                                let result = CaptureResult {
                                    raw: capture,
                                    template,
                                };
                                let _ = result_tx.send(result);
                            } else {
                                // Normal recognition mode.
                                if let Some(match_result) = self.recognizer.recognize(&capture, &self.templates) {
                                    let gesture_id = &match_result.gesture_id;
                                    let confidence = match_result.confidence;

                                    let threshold = self.gesture_configs.get(gesture_id)
                                        .and_then(|g| g.confidence_threshold)
                                        .unwrap_or(self.config.general.confidence_threshold);

                                    if confidence >= threshold {
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
                                        tracing::warn!("Gesture '{}' matched at {:.2} but below threshold {:.2}, ignoring", gesture_id, confidence, threshold);
                                        self.audio.play_error();
                                    }
                                } else {
                                    tracing::warn!("No gesture matched");
                                    self.audio.play_error();
                                }
                            }

                            if self.config.general.cursor_reset {
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
        self.input_source.start(tx)?;

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
                }
                TriggerSignal::GesturePoint(dx, dy) => {
                    accumulator.add_point(dx, dy);
                }
                TriggerSignal::GestureComplete => {
                    let capture = accumulator.finish();

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
        
        let _ = self.input_source.stop();

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
}
