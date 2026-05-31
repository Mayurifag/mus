use std::fs;

use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderValue, Method, Request, StatusCode},
    Router,
};
use mus_backend::{
    app::{app, test_state},
    state::AppState,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

struct TestApp {
    router: Router,
    state: AppState,
    _tmp: TempDir,
}

impl TestApp {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().to_path_buf();
        fs::create_dir_all(data_dir.join("music")).unwrap();
        fs::create_dir_all(data_dir.join("covers")).unwrap();
        let state = test_state(data_dir, Connection::open_in_memory().unwrap());
        let router = app(state.clone());
        Self {
            router,
            state,
            _tmp: tmp,
        }
    }

    async fn request(&self, method: Method, uri: &str, body: Body) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn json(&self, method: Method, uri: &str, payload: Value) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    fn insert_track(
        &self,
        id: i64,
        title: &str,
        artist: &str,
        file_path: &str,
        has_cover: bool,
        status: &str,
    ) {
        let conn = self.state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,processing_status,last_error) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id, title, artist, 180, file_path, 1000 + id, 2000 + id, has_cover, status, status.eq("ERROR").then_some("{\"message\":\"boom\"}")],
        )
        .unwrap();
    }
}

async fn body_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn healthcheck_contract() {
    let app = TestApp::new();
    let response = app
        .request(Method::GET, "/api/healthcheck.json", Body::empty())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["status"], "healthy");
    assert!(body["timestamp"].as_i64().is_some());
}

#[tokio::test]
async fn tracks_list_contract() {
    let app = TestApp::new();
    let file_path = app.state.music_dir.join("song.mp3");
    fs::write(&file_path, b"audio").unwrap();
    app.insert_track(
        1,
        "Song",
        "Artist",
        file_path.to_str().unwrap(),
        true,
        "COMPLETE",
    );

    let response = app
        .request(Method::GET, "/api/v1/tracks", Body::empty())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["title"], "Song");
    assert_eq!(body[0]["artist"], "Artist");
    assert_eq!(body[0]["filename"], "song.mp3");
    assert_eq!(body[0]["has_cover"], true);
    assert!(body[0].get("added_at").is_none());
    assert!(body[0].get("file_path").is_none());
    assert_eq!(
        body[0]["cover_small_url"],
        "/api/v1/tracks/1/covers/small.webp?v=2001"
    );
}

#[tokio::test]
async fn system_info_reports_missing_music_directory() {
    let app = TestApp::new();
    fs::remove_dir(app.state.music_dir.clone()).unwrap();

    let response = app
        .request(Method::GET, "/api/v1/system/info", Body::empty())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["music_dir"]["exists"], false);
    assert_eq!(
        body["music_dir"]["warning"],
        "Music directory is missing. Check the /app_data/music mount."
    );
}

#[tokio::test]
async fn tracks_stream_contract() {
    let app = TestApp::new();
    let missing = app
        .request(Method::GET, "/api/v1/tracks/404/stream", Body::empty())
        .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(missing).await,
        json!({"detail": "Track not found"})
    );

    let file_path = app.state.music_dir.join("song.mp3");
    fs::write(&file_path, b"abcdef").unwrap();
    app.insert_track(
        1,
        "Song",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );

    let response = app
        .request(Method::GET, "/api/v1/tracks/1/stream", Body::empty())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, max-age=86400"
    );
    assert_eq!(response.headers()[header::CONTENT_TYPE], "audio/mpeg");
    assert_eq!(response.headers()[header::CONTENT_LENGTH], "6");

    let ranged = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/tracks/1/stream")
                .header(header::RANGE, "bytes=0-2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ranged.status(), StatusCode::PARTIAL_CONTENT);

    let large_file_path = app.state.music_dir.join("large.mp3");
    fs::write(&large_file_path, vec![0; 1024 * 1024 + 7]).unwrap();
    app.insert_track(
        2,
        "Large",
        "Artist",
        large_file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    let large_no_range = app
        .request(Method::GET, "/api/v1/tracks/2/stream", Body::empty())
        .await;
    assert_eq!(large_no_range.status(), StatusCode::OK);
    assert_eq!(large_no_range.headers()[header::CONTENT_LENGTH], "1048583");

    let explicit_full_range = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/tracks/2/stream")
                .header(header::RANGE, "bytes=0-1048582")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(explicit_full_range.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        explicit_full_range.headers()[header::CONTENT_RANGE],
        "bytes 0-262143/1048583"
    );
    assert_eq!(
        explicit_full_range.headers()[header::CONTENT_LENGTH],
        "262144"
    );

    let open_ended_range = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/tracks/2/stream")
                .header(header::RANGE, "bytes=0-")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(open_ended_range.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        open_ended_range.headers()[header::CONTENT_RANGE],
        "bytes 0-262143/1048583"
    );
    assert_eq!(open_ended_range.headers()[header::CONTENT_LENGTH], "262144");

    let oversized_large_suffix_range = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/tracks/2/stream")
                .header(header::RANGE, "bytes=-1048583")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        oversized_large_suffix_range.status(),
        StatusCode::PARTIAL_CONTENT
    );
    assert_eq!(
        oversized_large_suffix_range.headers()[header::CONTENT_RANGE],
        "bytes 0-1048582/1048583"
    );
    assert_eq!(
        oversized_large_suffix_range.headers()[header::CONTENT_LENGTH],
        "1048583"
    );

    let oversized_suffix_range = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/tracks/1/stream")
                .header(header::RANGE, "bytes=-1024")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(oversized_suffix_range.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        oversized_suffix_range.headers()[header::CONTENT_RANGE],
        "bytes 0-5/6"
    );
    assert_eq!(
        oversized_suffix_range.headers()[header::CONTENT_LENGTH],
        "6"
    );

    let multiple_ranges = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/tracks/1/stream")
                .header(header::RANGE, "bytes=0-2,4-5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(multiple_ranges.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        multiple_ranges.headers()[header::CONTENT_RANGE],
        "bytes 0-2/6"
    );
    assert_eq!(multiple_ranges.headers()[header::CONTENT_LENGTH], "3");

    let invalid_range = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/tracks/1/stream")
                .header(header::RANGE, "bytes=999-1000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_range.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_ne!(
        invalid_range.headers().get(header::CONTENT_TYPE),
        Some(&HeaderValue::from_static("audio/mpeg"))
    );
}

#[tokio::test]
async fn covers_contract() {
    let app = TestApp::new();
    let missing = app
        .request(
            Method::GET,
            "/api/v1/tracks/1/covers/small.webp",
            Body::empty(),
        )
        .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(missing).await,
        json!({"detail": "Cover not found"})
    );

    fs::write(app.state.covers_dir.join("1_small.webp"), b"small").unwrap();
    let response = app
        .request(
            Method::GET,
            "/api/v1/tracks/1/covers/small.webp",
            Body::empty(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/webp");
}

#[tokio::test]
async fn track_update_delete_upload_contract() {
    let app = TestApp::new();
    let not_found = app
        .json(
            Method::PATCH,
            "/api/v1/tracks/404",
            json!({"title": "Nope"}),
        )
        .await;
    assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(not_found).await,
        json!({"detail": "Track not found"})
    );

    let file_path = app.state.music_dir.join("song.mp3");
    fs::write(&file_path, b"audio").unwrap();
    app.insert_track(
        1,
        "Song",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );

    let unchanged = app
        .json(
            Method::PATCH,
            "/api/v1/tracks/1",
            json!({"title": "Song", "artist": "Artist"}),
        )
        .await;
    assert_eq!(unchanged.status(), StatusCode::OK);
    assert_eq!(body_json(unchanged).await, json!({"status": "no_changes"}));

    let duplicate_path = app.state.music_dir.join("Artist - Taken.mp3");
    fs::write(&duplicate_path, b"taken").unwrap();
    let duplicate_rename = app
        .json(
            Method::PATCH,
            "/api/v1/tracks/1",
            json!({"title": "Taken", "artist": "Artist", "rename_file": true}),
        )
        .await;
    assert_eq!(duplicate_rename.status(), StatusCode::CONFLICT);
    assert_eq!(
        body_json(duplicate_rename).await,
        json!({"detail": "A file with this name already exists"})
    );

    let boundary = "mus-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nTitle\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"artist\"\r\n\r\nArtist\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"song.txt\"\r\nContent-Type: text/plain\r\n\r\ntext\r\n--{boundary}--\r\n"
    );
    let upload = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/tracks/upload")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upload.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(upload).await,
        json!({"detail": "Unsupported file format. Only MP3, FLAC, and WAV are supported."})
    );

    let duplicate_upload_body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nTaken\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"artist\"\r\n\r\nArtist\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"song.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\naudio\r\n--{boundary}--\r\n"
    );
    let duplicate_upload = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/tracks/upload")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(duplicate_upload_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(duplicate_upload.status(), StatusCode::CONFLICT);
    assert_eq!(
        body_json(duplicate_upload).await,
        json!({"detail": "A file with this name already exists"})
    );

    let deleted = app
        .request(Method::DELETE, "/api/v1/tracks/1", Body::empty())
        .await;
    assert_eq!(deleted.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn player_state_contract() {
    let app = TestApp::new();
    let default_state = app
        .request(Method::GET, "/api/v1/player/state", Body::empty())
        .await;
    assert_eq!(default_state.status(), StatusCode::OK);
    assert_eq!(
        body_json(default_state).await,
        json!({
            "current_track_id": null,
            "progress_seconds": 0.0,
            "volume_level": 1.0,
            "is_muted": false,
            "is_shuffle": false,
            "is_repeat": false
        })
    );

    let payload = json!({
        "current_track_id": 10,
        "progress_seconds": 45.5,
        "volume_level": 0.7,
        "is_muted": true,
        "is_shuffle": false,
        "is_repeat": true
    });
    let saved = app
        .json(Method::POST, "/api/v1/player/state", payload.clone())
        .await;
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(body_json(saved).await, payload);
}

#[tokio::test]
async fn system_contract() {
    let app = TestApp::new();
    let permissions = app
        .request(Method::GET, "/api/v1/system/permissions", Body::empty())
        .await;
    assert_eq!(permissions.status(), StatusCode::OK);
    assert_eq!(
        body_json(permissions).await,
        json!({"can_write_music_files": true})
    );

    let info = app
        .request(Method::GET, "/api/v1/system/info", Body::empty())
        .await;
    assert_eq!(info.status(), StatusCode::OK);
    let body = body_json(info).await;
    assert_eq!(body["app_date"], "2026-05-17");
    assert_eq!(body["commit_sha"], "test-sha");
    assert_eq!(body["yt_dlp_version"], "test-version");

    let rescan = app
        .request(Method::POST, "/api/v1/system/rescan", Body::empty())
        .await;
    assert_eq!(rescan.status(), StatusCode::OK);
    assert_eq!(body_json(rescan).await, json!({"status": "ok"}));
}

#[tokio::test]
async fn downloads_contract() {
    let app = TestApp::new();
    *app.state.download_lock.lock().unwrap() = true;
    let locked = app
        .json(
            Method::POST,
            "/api/v1/downloads/url",
            json!({"url": "https://example.com/video"}),
        )
        .await;
    assert_eq!(locked.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        body_json(locked).await,
        json!({"detail": "Download already in progress"})
    );

    let confirm_locked = app
        .json(
            Method::POST,
            "/api/v1/downloads/confirm",
            json!({"url": "https://example.com/video", "title": "Song", "artist": "Artist"}),
        )
        .await;
    assert_eq!(confirm_locked.status(), StatusCode::TOO_MANY_REQUESTS);
    *app.state.download_lock.lock().unwrap() = false;

    let duplicate_download_path = app.state.music_dir.join("Artist - Existing.mp3");
    fs::write(&duplicate_download_path, b"existing").unwrap();
    let duplicate_download = app
        .json(
            Method::POST,
            "/api/v1/downloads/confirm",
            json!({"url": "https://example.com/video", "title": "Existing", "artist": "Artist"}),
        )
        .await;
    assert_eq!(duplicate_download.status(), StatusCode::CONFLICT);
    assert_eq!(
        body_json(duplicate_download).await,
        json!({"detail": "A file with this name already exists"})
    );

    let invalid_metadata = app
        .json(Method::POST, "/api/v1/downloads/metadata", json!({}))
        .await;
    assert_eq!(invalid_metadata.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn events_and_errors_contract() {
    let app = TestApp::new();
    let trigger = app
        .json(
            Method::POST,
            "/api/v1/events/trigger",
            json!({
                "message_to_show": "Updated",
                "message_level": "info",
                "action_key": "track_updated",
                "action_payload": {"id": 1}
            }),
        )
        .await;
    assert_eq!(trigger.status(), StatusCode::OK);
    assert_eq!(body_json(trigger).await, json!({"status": "ok"}));

    let sse = app
        .request(Method::GET, "/api/v1/events/track-updates", Body::empty())
        .await;
    assert_eq!(sse.status(), StatusCode::OK);
    assert_eq!(sse.headers()[header::CONTENT_TYPE], "text/event-stream");

    let file_path = app.state.music_dir.join("errored.mp3");
    fs::write(&file_path, b"audio").unwrap();
    app.insert_track(
        1,
        "Bad",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "ERROR",
    );
    let errors = app
        .request(Method::GET, "/api/v1/errors/tracks", Body::empty())
        .await;
    assert_eq!(errors.status(), StatusCode::OK);
    let body = body_json(errors).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["title"], "Bad");

    let requeue_missing = app
        .request(
            Method::POST,
            "/api/v1/errors/tracks/404/requeue",
            Body::empty(),
        )
        .await;
    assert_eq!(requeue_missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(requeue_missing).await,
        json!({"detail": "Track not found"})
    );
}
