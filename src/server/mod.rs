pub mod handlers;

use std::sync::Arc;
use axum::{
    routing::get,
    Router,
    extract::{State, WebSocketUpgrade},
    response::{Html, IntoResponse},
};
use tokio::sync::{Mutex, mpsc};
use crate::config::{Config, GestureProfile};
use crate::pipeline::CaptureRequest;

pub struct ServerState {
    pub config: Config,
    pub gesture_profile: GestureProfile,
}

pub type SharedState = Arc<Mutex<ServerState>>;

pub async fn start(
    port: u16,
    state: SharedState,
    capture_tx: mpsc::Sender<CaptureRequest>,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index_html))
        .route("/ws", get(ws_handler))
        .with_state((state, capture_tx));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("Web UI listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_html() -> Html<&'static str> {
    Html(include_str!("../../assets/index.html"))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State((state, capture_tx)): State<(SharedState, mpsc::Sender<CaptureRequest>)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handlers::handle_socket(socket, state, capture_tx))
}
