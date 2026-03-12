use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tracing::info;

use crate::{config::PipelineConfig, error::AgentFlowError};

use super::hub::EventHub;

// Embed the UI at compile time so the binary is self-contained.
const INDEX_HTML: &str = include_str!("../../static/index.html");

#[derive(Clone)]
struct AppState {
    hub: Arc<EventHub>,
    config: PipelineConfig,
}

/// Start the observability HTTP server on `port` in a background task.
///
/// Serves:
/// - `GET /`           — self-contained agent graph UI
/// - `GET /api/config` — pipeline config as JSON (used by the UI to build the graph)
/// - `GET /ws`         — WebSocket stream of `PipelineEvent` JSON
pub async fn start(
    hub: Arc<EventHub>,
    config: PipelineConfig,
    port: u16,
) -> Result<(), AgentFlowError> {
    let state = AppState { hub, config };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/config", get(config_handler))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("observability UI  ->  http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(AgentFlowError::Io)?;

    axum::serve(listener, app)
        .await
        .map_err(|e| AgentFlowError::Agent(e.to_string()))?;

    Ok(())
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn config_handler(State(state): State<AppState>) -> Json<Value> {
    Json(serde_json::to_value(&state.config).unwrap_or(Value::Null))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state.hub))
}

async fn handle_socket(socket: WebSocket, hub: Arc<EventHub>) {
    let mut rx = hub.subscribe();
    let (mut sender, mut receiver) = socket.split();

    // Forward hub events to the WebSocket client
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(_) => continue,
                    };
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Drain (and discard) any messages from the browser
    while let Some(Ok(_)) = receiver.next().await {}

    send_task.abort();
}
