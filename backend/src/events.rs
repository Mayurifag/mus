use async_stream::stream;
use axum::{extract::State, response::sse::Event, response::sse::KeepAlive, response::Sse, Json};
use futures_util::Stream;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::{models::MusEvent, state::AppState};

pub async fn track_updates(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.events.subscribe();
    tracing::info!(
        receivers = state.events.receiver_count(),
        "sse track updates connected"
    );
    Sse::new(stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    tracing::info!(action = event.action_key.as_deref().unwrap_or(""), "sse track update event sent");
                    yield Ok(Event::default().json_data(event).unwrap());
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "sse track updates lagged");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!("sse track updates closed");
                    break;
                }
            }
        }
    })
    .keep_alive(KeepAlive::default())
}

pub async fn trigger_event(
    State(state): State<AppState>,
    Json(event): Json<MusEvent>,
) -> Json<Value> {
    let _ = state.events.send(event);
    Json(json!({"status": "ok"}))
}

pub fn broadcast(
    state: &AppState,
    action: &str,
    message: Option<&str>,
    level: Option<&str>,
    payload: Option<Value>,
) {
    let event = MusEvent {
        message_to_show: message.map(str::to_string),
        message_level: level.map(str::to_string),
        action_key: Some(action.to_string()),
        action_payload: payload,
    };
    match state.events.send(event) {
        Ok(receivers) => tracing::info!(action, receivers, "broadcast event queued"),
        Err(_) => tracing::debug!(action, "broadcast event skipped; no sse receivers"),
    }
}
