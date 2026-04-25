use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::config::{Config, GestureConfig};
use crate::pipeline::CaptureRequest;
use super::SharedState;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    GetConfig,
    ListGestures,
    SaveGesture { gesture: GestureConfig },
    DeleteGesture { name: String },
    SetConfig { config: Config },
    StartCapture,
    CancelCapture,
    Reload,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Config { data: Config },
    Gestures { data: Vec<GestureConfig> },
    CaptureResult { raw: crate::types::GestureCapture, processed: crate::types::GestureTemplate },
    CaptureCancelled,
    Ok,
    Error { message: String },
}

pub async fn handle_socket(
    socket: WebSocket,
    state: SharedState,
    capture_tx: mpsc::Sender<CaptureRequest>,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(32);
    
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut current_capture: Option<oneshot::Sender<()>> = None;

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_msg) => {
                    match client_msg {
                        ClientMessage::GetConfig => {
                            let state_guard = state.lock().await;
                            let _ = tx.send(ServerMessage::Config { data: state_guard.config.clone() }).await;
                        }
                        ClientMessage::ListGestures => {
                            let state_guard = state.lock().await;
                            let _ = tx.send(ServerMessage::Gestures { data: state_guard.gesture_profile.gestures.clone() }).await;
                        }
                        ClientMessage::SaveGesture { gesture } => {
                            let mut state_guard = state.lock().await;
                            let profile_name = state_guard.config.general.gesture_profile.clone();
                            
                            let mut found = false;
                            for existing in &mut state_guard.gesture_profile.gestures {
                                if existing.name == gesture.name {
                                    *existing = gesture.clone();
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                state_guard.gesture_profile.gestures.push(gesture);
                            }
                            
                            if let Err(e) = crate::config::save_gesture_profile(&profile_name, &state_guard.gesture_profile) {
                                let _ = tx.send(ServerMessage::Error { message: e.to_string() }).await;
                            } else {
                                let _ = tx.send(ServerMessage::Ok).await;
                            }
                        }
                        ClientMessage::DeleteGesture { name } => {
                            let mut state_guard = state.lock().await;
                            let profile_name = state_guard.config.general.gesture_profile.clone();
                            state_guard.gesture_profile.gestures.retain(|g| g.name != name);
                            
                            if let Err(e) = crate::config::save_gesture_profile(&profile_name, &state_guard.gesture_profile) {
                                let _ = tx.send(ServerMessage::Error { message: e.to_string() }).await;
                            } else {
                                let _ = tx.send(ServerMessage::Ok).await;
                            }
                        }
                        ClientMessage::SetConfig { config } => {
                            let mut state_guard = state.lock().await;
                            state_guard.config = config.clone();
                            
                            let config_dir = crate::config::get_config_dir().unwrap_or_default();
                            let config_path = config_dir.join("config.toml");
                            
                            if let Ok(toml_str) = toml::to_string_pretty(&config) {
                                if let Err(e) = std::fs::write(&config_path, toml_str) {
                                    let _ = tx.send(ServerMessage::Error { message: e.to_string() }).await;
                                } else {
                                    let _ = tx.send(ServerMessage::Ok).await;
                                }
                            } else {
                                let _ = tx.send(ServerMessage::Error { message: "Serialization failed".to_string() }).await;
                            }
                        }
                        ClientMessage::StartCapture => {
                            let (res_tx, res_rx) = oneshot::channel();
                            if capture_tx.send(CaptureRequest { result_tx: res_tx }).await.is_ok() {
                                let tx_clone = tx.clone();
                                let (abort_tx, mut abort_rx) = oneshot::channel::<()>();
                                current_capture = Some(abort_tx);
                                
                                tokio::spawn(async move {
                                    tokio::select! {
                                        _ = &mut abort_rx => {
                                            // aborted
                                        }
                                        res = res_rx => {
                                            match res {
                                                Ok(result) => {
                                                    let _ = tx_clone.send(ServerMessage::CaptureResult {
                                                        raw: result.raw,
                                                        processed: result.template,
                                                    }).await;
                                                }
                                                Err(_) => {
                                                    let _ = tx_clone.send(ServerMessage::CaptureCancelled).await;
                                                }
                                            }
                                        }
                                    }
                                });
                            } else {
                                let _ = tx.send(ServerMessage::Error { message: "Capture channel closed".to_string() }).await;
                            }
                        }
                        ClientMessage::CancelCapture => {
                            if let Some(abort_tx) = current_capture.take() {
                                let _ = abort_tx.send(());
                            }
                            let _ = tx.send(ServerMessage::CaptureCancelled).await;
                        }
                        ClientMessage::Reload => {
                            let mut state_guard = state.lock().await;
                            let profile_name = state_guard.config.general.gesture_profile.clone();
                            match crate::config::load_gesture_profile(&profile_name) {
                                Ok(profile) => {
                                    state_guard.gesture_profile = profile;
                                    let _ = tx.send(ServerMessage::Ok).await;
                                }
                                Err(e) => {
                                    let _ = tx.send(ServerMessage::Error { message: e.to_string() }).await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(ServerMessage::Error { message: e.to_string() }).await;
                }
            }
        }
    }
}
