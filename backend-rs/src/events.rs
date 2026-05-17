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
    Sse::new(stream! {
        loop {
            match rx.recv().await {
                Ok(event) => yield Ok(Event::default().json_data(event).unwrap()),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
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
    let _ = state.events.send(MusEvent {
        message_to_show: message.map(str::to_string),
        message_level: level.map(str::to_string),
        action_key: Some(action.to_string()),
        action_payload: payload,
    });
}
