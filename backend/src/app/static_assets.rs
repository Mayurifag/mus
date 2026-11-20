use std::path::{Component, Path, PathBuf};

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tokio::fs;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::state::AppState;

pub(super) async fn static_asset(
    State(state): State<AppState>,
    request: Request,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_owned();
    if path.starts_with("/api/") {
        return Ok((StatusCode::NOT_FOUND, Json(json!({"detail": "Not Found"}))).into_response());
    }
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    let Some(static_dir) = state.static_dir else {
        return Err(StatusCode::NOT_FOUND);
    };
    let requested = static_path(&static_dir, &path).ok_or(StatusCode::NOT_FOUND)?;
    let file_path = if is_file(&requested).await {
        requested
    } else if path.starts_with("/_app/") {
        return Err(StatusCode::NOT_FOUND);
    } else {
        static_dir.join("index.html")
    };
    if !is_file(&file_path).await {
        return Err(StatusCode::NOT_FOUND);
    }

    let Ok(response) = ServeFile::new(file_path).oneshot(request).await;
    let mut response = response.into_response();
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

async fn is_file(path: &Path) -> bool {
    fs::metadata(path).await.is_ok_and(|meta| meta.is_file())
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
