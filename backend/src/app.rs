use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use tokio::fs as async_fs;
use tower::ServiceExt;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeFile,
};

use crate::{
    artwork::{search_artwork, stream_artwork},
    db::init_db,
    downloads::{confirm_download, fetch_metadata, start_download},
    errors::{get_errored_tracks, requeue_track},
    events::{track_updates, trigger_event},
    hls::{get_hls_playlist, get_hls_segment},
    player::{get_player_state, save_player_state},
    state::AppState,
    system::{get_permissions, get_system_info, rescan, update_yt_dlp},
    tracks::{
        delete_track, get_cover_original, get_cover_small, get_tracks, update_track, upload_track,
    },
    util::now,
};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/healthcheck.json", get(healthcheck))
        .route("/api/v1/tracks", get(get_tracks))
        .route("/api/v1/artwork/search", get(search_artwork))
        .route("/api/v1/artwork/search/stream", get(stream_artwork))
        .route("/api/v1/tracks/upload", post(upload_track))
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
        .layer(middleware::from_fn(request_logging))
        .layer(cors_layer())
        .with_state(state)
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin
                .to_str()
                .map(|origin| {
                    matches!(
                        origin,
                        "http://localhost:5173"
                            | "http://localhost:5174"
                            | "http://127.0.0.1:5173"
                            | "http://127.0.0.1:5174"
                    )
                })
                .unwrap_or(false)
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

pub fn test_state(data_dir: PathBuf, mut conn: Connection) -> AppState {
    let music_dir = data_dir.join("music");
    let cache_dir = data_dir.join(".cache");
    let covers_dir = cache_dir.join("covers");
    std::fs::create_dir_all(&music_dir).unwrap();
    std::fs::create_dir_all(&covers_dir).unwrap();
    init_db(&mut conn).unwrap();
    let (events, _) = tokio::sync::broadcast::channel(100);
    AppState {
        db: Arc::new(Mutex::new(conn)),
        data_dir,
        static_dir: None,
        music_dir,
        covers_dir,
        events,
        download_lock: Arc::new(Mutex::new(false)),
        mutation_locks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        app_date: "2026-05-17".into(),
        commit_sha: Some("test-sha".into()),
        yt_dlp_version: Arc::new(Mutex::new(Some("test-version".into()))),
    }
}

async fn healthcheck() -> Json<Value> {
    Json(json!({"status": "healthy", "timestamp": now()}))
}

async fn static_asset(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let path = uri.path();
    if path.starts_with("/api/") {
        return Ok((StatusCode::NOT_FOUND, Json(json!({"detail": "Not Found"}))).into_response());
    }
    let Some(static_dir) = state.static_dir else {
        return Err(StatusCode::NOT_FOUND);
    };

    let requested = static_path(&static_dir, path).ok_or(StatusCode::NOT_FOUND)?;
    let file_path = if async_fs::metadata(&requested)
        .await
        .map(|meta| meta.is_file())
        .unwrap_or(false)
    {
        requested
    } else {
        static_dir.join("index.html")
    };
    if !async_fs::metadata(&file_path)
        .await
        .map(|meta| meta.is_file())
        .unwrap_or(false)
    {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut request = Request::new(Body::empty());
    *request.headers_mut() = headers;
    let mut response = ServeFile::new(file_path)
        .oneshot(request)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_response();

    response.headers_mut().insert(
        header::CACHE_CONTROL,
        if path.starts_with("/_app/immutable/") {
            HeaderValue::from_static("public, immutable, max-age=31536000")
        } else {
            HeaderValue::from_static("no-cache")
        },
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    Ok(response)
}

fn static_path(static_dir: &Path, uri_path: &str) -> Option<PathBuf> {
    let relative = uri_path.trim_start_matches('/');
    if relative.is_empty() {
        return Some(static_dir.join("index.html"));
    }

    let mut path = static_dir.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(value) => path.push(value),
            _ => return None,
        }
    }
    Some(path)
}
