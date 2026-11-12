use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    extract::Request,
    http::{header, HeaderValue, Method},
    middleware::{self, Next},
    response::Response,
    routing::{get, patch, post},
    Json, Router,
};
use serde_json::{json, Value};
use tower_http::{
    compression::{
        predicate::{DefaultPredicate, NotForContentType, Predicate},
        CompressionLayer,
    },
    cors::{AllowOrigin, CorsLayer},
};

mod static_assets;
use static_assets::static_asset;

use crate::{
    artwork::{search_artwork, stream_artwork},
    downloads::{confirm_download, fetch_metadata, start_download},
    events::{track_updates, trigger_event},
    hls::{get_hls_playlist, get_hls_segment},
    player::{get_player_state, save_player_state},
    state::AppState,
    system::{get_permissions, get_system_info, rescan, update_yt_dlp},
    time::now,
    tracks::{
        delete_track, get_cover_original, get_cover_small, get_cut_info, get_cut_preview,
        get_errored_tracks, get_tracks, requeue_track, update_track, upload_track,
    },
};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Builds the HTTP API and optional static frontend using shared application state.
///
/// The router has no built-in authentication; deployments must provide external
/// authentication. This function does not start scanning, watching, or a listener.
///
/// ```
/// # fn example(state: mus_backend::state::AppState) {
/// let router = mus_backend::app::app(state);
/// # let _ = router;
/// # }
/// ```
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/healthcheck.json", get(healthcheck))
        .route("/api/v1/tracks", get(get_tracks))
        .route("/api/v1/artwork/search", get(search_artwork))
        .route("/api/v1/artwork/search/stream", get(stream_artwork))
        .route("/api/v1/tracks/upload", post(upload_track))
        .route("/api/v1/tracks/{id}/cut-info", get(get_cut_info))
        .route("/api/v1/tracks/{id}/cut-preview", post(get_cut_preview))
        .route(
            "/api/v1/tracks/{id}",
            patch(update_track).delete(delete_track),
        )
        .route(
            "/api/v1/tracks/{id}/hls/{cache_key}/index.m3u8",
            get(get_hls_playlist),
        )
        .route(
            "/api/v1/tracks/{id}/hls/{cache_key}/{segment}",
            get(get_hls_segment),
        )
        .route(
            "/api/v1/tracks/{id}/covers/small.webp",
            get(get_cover_small),
        )
        .route(
            "/api/v1/tracks/{id}/covers/original.webp",
            get(get_cover_original),
        )
        .route(
            "/api/v1/player/state",
            get(get_player_state).post(save_player_state),
        )
        .route("/api/v1/system/permissions", get(get_permissions))
        .route("/api/v1/system/info", get(get_system_info))
        .route("/api/v1/system/yt-dlp/update", post(update_yt_dlp))
        .route("/api/v1/system/rescan", post(rescan))
        .route("/api/v1/downloads/metadata", post(fetch_metadata))
        .route("/api/v1/downloads/url", post(start_download))
        .route("/api/v1/downloads/confirm", post(confirm_download))
        .route("/api/v1/events/track-updates", get(track_updates))
        .route("/api/v1/events/trigger", post(trigger_event))
        .route("/api/v1/errors/tracks", get(get_errored_tracks))
        .route("/api/v1/errors/tracks/{id}/requeue", post(requeue_track))
        .fallback(static_asset)
        .layer(
            CompressionLayer::new().compress_when(
                DefaultPredicate::new()
                    .and(NotForContentType::const_new("audio/"))
                    // Compression buffers incremental artwork results.
                    .and(NotForContentType::const_new("application/x-ndjson")),
            ),
        )
        .layer(middleware::from_fn(request_logging))
        .layer(cors_layer())
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin.to_str().is_ok_and(|origin| {
                matches!(
                    origin,
                    "http://localhost:5173"
                        | "http://localhost:5174"
                        | "http://127.0.0.1:5173"
                        | "http://127.0.0.1:5174"
                )
            })
        }))
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true)
}

async fn request_logging(request: Request, next: Next) -> Response {
    let request_id_header_name = header::HeaderName::from_static("x-request-id");
    let request_id = request
        .headers()
        .get(&request_id_header_name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| format!("req-{}", REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)));
    let request_id_header = HeaderValue::from_str(&request_id).ok();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    if let Some(value) = request_id_header {
        response.headers_mut().insert(request_id_header_name, value);
    }

    if response.status().is_server_error() {
        tracing::warn!(
            request_id,
            method = %method,
            path,
            status = response.status().as_u16(),
            "request failed"
        );
    }

    response
}

async fn healthcheck() -> Json<Value> {
    Json(json!({"status": "healthy", "timestamp": now()}))
}
