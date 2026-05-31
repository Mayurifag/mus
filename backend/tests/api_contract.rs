use std::fs;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
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

const TRACK_ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const MISSING_TRACK_ID: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

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
        id: &str,
        title: &str,
        artist: &str,
        file_path: &str,
        has_cover: bool,
        status: &str,
    ) {
        let conn = self.state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,content_hash,processing_status,last_error) VALUES (?1,?2,?3,?4,?5,1000,2000,?6,?1,?7,?8)",
            params![id, title, artist, 180, file_path, has_cover, status, status.eq("ERROR").then_some("{\"message\":\"boom\"}")],
        )
        .unwrap();
    }
}

async fn body_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn body_text(response: axum::response::Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

fn write_silent_wav(path: &std::path::Path) {
    let sample_rate = 8_000u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let samples = sample_rate;
    let data_len = samples * channels as u32 * bits_per_sample as u32 / 8;
    let mut data = Vec::new();
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + data_len).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&channels.to_le_bytes());
    data.extend_from_slice(&sample_rate.to_le_bytes());
    data.extend_from_slice(
        &(sample_rate * channels as u32 * bits_per_sample as u32 / 8).to_le_bytes(),
    );
    data.extend_from_slice(&(channels * bits_per_sample / 8).to_le_bytes());
    data.extend_from_slice(&bits_per_sample.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&data_len.to_le_bytes());
    data.resize(data.len() + data_len as usize, 0);
    fs::write(path, data).unwrap();
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
async fn cors_preflight_allows_credentialed_dev_origin() {
    let app = TestApp::new();
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/player/state")
                .header(header::ORIGIN, "http://localhost:5174")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "http://localhost:5174"
    );
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
        "true"
    );
}

#[tokio::test]
async fn cors_preflight_does_not_allow_unknown_origin() {
    let app = TestApp::new();
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/player/state")
                .header(header::ORIGIN, "https://example.com")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());
}

#[tokio::test]
async fn tracks_list_contract() {
    let app = TestApp::new();
    let file_path = app.state.music_dir.join("song.mp3");
    fs::write(&file_path, b"audio").unwrap();
    app.insert_track(
        TRACK_ID,
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
        format!("/api/v1/tracks/{TRACK_ID}/covers/small.webp?v={TRACK_ID}")
    );
    assert_eq!(
        body[0]["cover_original_url"],
        format!("/api/v1/tracks/{TRACK_ID}/covers/original.webp?v={TRACK_ID}")
    );
    assert_eq!(
        body[0]["hls_url"],
        format!("/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/index.m3u8")
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
async fn tracks_hls_contract() {
    let app = TestApp::new();
    let missing = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{MISSING_TRACK_ID}/hls/2001/index.m3u8"),
            Body::empty(),
        )
        .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(missing).await,
        json!({"detail": "Track not found"})
    );

    let file_path = app.state.music_dir.join("song.wav");
    write_silent_wav(&file_path);
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );

    let removed_stream = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/stream"),
            Body::empty(),
        )
        .await;
    assert_eq!(removed_stream.status(), StatusCode::NOT_FOUND);

    let stale = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/hls/stale/index.m3u8"),
            Body::empty(),
        )
        .await;
    assert_eq!(stale.status(), StatusCode::NOT_FOUND);

    let stale_cache_dir = app
        .state
        .data_dir
        .join(format!(".cache/hls/{TRACK_ID}/stale"));
    fs::create_dir_all(&stale_cache_dir).unwrap();
    fs::write(stale_cache_dir.join("segment-00000.m4s"), b"stale").unwrap();

    let playlist = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/index.m3u8"),
            Body::empty(),
        )
        .await;
    assert_eq!(playlist.status(), StatusCode::OK);
    assert_eq!(
        playlist.headers()[header::CONTENT_TYPE],
        "application/vnd.apple.mpegurl"
    );
    assert_eq!(playlist.headers()[header::CACHE_CONTROL], "no-cache");
    let playlist_body = body_text(playlist).await;
    assert!(playlist_body.contains("#EXTM3U"));
    assert!(playlist_body.contains("init.mp4"));
    assert!(playlist_body.contains("segment-00000.m4s"));
    assert!(!stale_cache_dir.exists());

    let init = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/init.mp4"),
            Body::empty(),
        )
        .await;
    assert_eq!(init.status(), StatusCode::OK);
    assert_eq!(init.headers()[header::CONTENT_TYPE], "audio/mp4");

    let segment = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/segment-00000.m4s"),
            Body::empty(),
        )
        .await;
    assert_eq!(segment.status(), StatusCode::OK);
    assert_eq!(segment.headers()[header::CONTENT_TYPE], "audio/mp4");
    assert_eq!(
        segment.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );

    let invalid_segment = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/not-a-segment.ts"),
            Body::empty(),
        )
        .await;
    assert_eq!(invalid_segment.status(), StatusCode::NOT_FOUND);

    fs::write(
        app.state.covers_dir.join(format!("{TRACK_ID}_small.webp")),
        b"small",
    )
    .unwrap();
    fs::write(
        app.state
            .covers_dir
            .join(format!("{TRACK_ID}_original.webp")),
        b"original",
    )
    .unwrap();
    let deleted = app
        .request(
            Method::DELETE,
            &format!("/api/v1/tracks/{TRACK_ID}"),
            Body::empty(),
        )
        .await;
    assert_eq!(deleted.status(), StatusCode::ACCEPTED);
    assert!(!app
        .state
        .data_dir
        .join(format!(".cache/hls/{TRACK_ID}"))
        .exists());
    assert!(!app
        .state
        .covers_dir
        .join(format!("{TRACK_ID}_small.webp"))
        .exists());
    assert!(!app
        .state
        .covers_dir
        .join(format!("{TRACK_ID}_original.webp"))
        .exists());
}

#[tokio::test]
async fn covers_contract() {
    let app = TestApp::new();
    let missing = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/covers/small.webp"),
            Body::empty(),
        )
        .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(missing).await,
        json!({"detail": "Cover not found"})
    );

    fs::write(
        app.state.covers_dir.join(format!("{TRACK_ID}_small.webp")),
        b"small",
    )
    .unwrap();
    let response = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/covers/small.webp"),
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
            &format!("/api/v1/tracks/{MISSING_TRACK_ID}"),
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
        TRACK_ID,
        "Song",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );

    let unchanged = app
        .json(
            Method::PATCH,
            &format!("/api/v1/tracks/{TRACK_ID}"),
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
            &format!("/api/v1/tracks/{TRACK_ID}"),
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
        .request(
            Method::DELETE,
            &format!("/api/v1/tracks/{TRACK_ID}"),
            Body::empty(),
        )
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
        "current_track_id": TRACK_ID,
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
                "action_payload": {"id": TRACK_ID}
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
        TRACK_ID,
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
            &format!("/api/v1/errors/tracks/{MISSING_TRACK_ID}/requeue"),
            Body::empty(),
        )
        .await;
    assert_eq!(requeue_missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(requeue_missing).await,
        json!({"detail": "Track not found"})
    );
}
