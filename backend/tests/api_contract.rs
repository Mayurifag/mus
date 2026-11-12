//! HTTP contracts exercised through the application router.

use std::{fs, io::Read};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use flate2::read::GzDecoder;
use mus_backend::{app::app, state::AppState};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

mod common;
use common::test_state;

const TRACK_ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const OTHER_TRACK_ID: &str = "2222222222222222222222222222222222222222222222222222222222222222";
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

    async fn get_with_gzip(&self, uri: &str) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(header::ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
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

    fn set_tags(&self, track_id: &str, tags: &[&str]) {
        let conn = self.state.db.lock().unwrap();
        conn.execute(
            "DELETE FROM track_tag WHERE track_id = ?1",
            params![track_id],
        )
        .unwrap();
        for tag in tags {
            conn.execute(
                "INSERT INTO track_tag (track_id, tag_id) SELECT ?1, id FROM tag WHERE name = ?2",
                params![track_id, tag],
            )
            .unwrap();
        }
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

async fn gzip_body(response: axum::response::Response) -> Vec<u8> {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let mut decoded = Vec::new();
    GzDecoder::new(bytes.as_ref())
        .read_to_end(&mut decoded)
        .unwrap();
    decoded
}

#[tokio::test]
async fn gzip_compresses_track_data_without_changing_it() {
    let app = TestApp::new();
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        "/music/song.mp3",
        false,
        "COMPLETE",
    );

    let plain = app
        .request(Method::GET, "/api/v1/tracks", Body::empty())
        .await;
    assert!(!plain.headers().contains_key(header::CONTENT_ENCODING));
    let expected = body_json(plain).await;

    let compressed = app.get_with_gzip("/api/v1/tracks").await;
    assert_eq!(compressed.headers()[header::CONTENT_ENCODING], "gzip");
    assert!(compressed.headers()[header::VARY]
        .to_str()
        .unwrap()
        .contains("accept-encoding"));
    let decoded: Value = serde_json::from_slice(&gzip_body(compressed).await).unwrap();
    assert_eq!(decoded, expected);
}

#[tokio::test]
async fn gzip_compresses_static_javascript() {
    let test_app = TestApp::new();
    let static_dir = test_app._tmp.path().join("static");
    let asset_dir = static_dir.join("_app/immutable");
    fs::create_dir_all(&asset_dir).unwrap();
    let script = "console.log('music');\n".repeat(100);
    fs::write(asset_dir.join("app.js"), &script).unwrap();
    let mut state = test_app.state.clone();
    state.static_dir = Some(static_dir);
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/_app/immutable/app.js")
                .header(header::ACCEPT_ENCODING, "gzip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, immutable, max-age=31536000"
    );
    assert_eq!(gzip_body(response).await, script.as_bytes());
}

#[tokio::test]
async fn gzip_skips_artwork_search_stream() {
    let app = TestApp::new();
    let response = app
        .get_with_gzip("/api/v1/artwork/search/stream?title=Song&artist=Artist")
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/x-ndjson"
    );
    assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
}

#[tokio::test]
async fn gzip_skips_event_streams_and_hls_audio() {
    let app = TestApp::new();
    let events = app.get_with_gzip("/api/v1/events/track-updates").await;
    assert!(!events.headers().contains_key(header::CONTENT_ENCODING));
    assert!(events.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));

    let source = app.state.music_dir.join("song.mp3");
    fs::write(&source, b"audio").unwrap();
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        source.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    let hls_dir = app
        .state
        .data_dir
        .join(".cache/hls")
        .join(TRACK_ID)
        .join(TRACK_ID);
    fs::create_dir_all(&hls_dir).unwrap();
    let audio = b"audio data".repeat(10);
    fs::write(hls_dir.join("init.mp4"), &audio).unwrap();
    let response = app
        .get_with_gzip(&format!(
            "/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/init.mp4"
        ))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(header::CONTENT_ENCODING));
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        audio
    );
}

#[tokio::test]
async fn missing_cut_tracks_do_not_allocate_mutation_locks() {
    let app = TestApp::new();
    for id in ["not-a-track", MISSING_TRACK_ID] {
        let info = app
            .request(
                Method::GET,
                &format!("/api/v1/tracks/{id}/cut-info"),
                Body::empty(),
            )
            .await;
        assert_eq!(info.status(), StatusCode::NOT_FOUND);
        let preview = app
            .json(
                Method::POST,
                &format!("/api/v1/tracks/{id}/cut-preview"),
                json!({"remove_section": {"start_seconds": 0.1, "end_seconds": 0.2}}),
            )
            .await;
        assert_eq!(preview.status(), StatusCode::NOT_FOUND);
    }
    assert!(app.state.mutation_locks.lock().await.is_empty());
}

#[tokio::test]
async fn mutation_locks_serialize_work_without_retaining_expired_keys() {
    let app = TestApp::new();
    let guard = app.state.mutation_lock("same-track".into()).await;
    let mut waiting = Box::pin(app.state.mutation_lock("same-track".into()));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut waiting)
            .await
            .is_err()
    );
    drop(guard);
    drop(
        tokio::time::timeout(std::time::Duration::from_secs(1), waiting)
            .await
            .unwrap(),
    );
    for i in 0..100 {
        let _guard = app.state.mutation_lock(format!("track-{i}")).await;
    }
    assert_eq!(app.state.mutation_locks.lock().await.len(), 1);
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
    assert_eq!(body[0]["tags"], json!([]));
}

#[tokio::test]
async fn category_contract() {
    let app = TestApp::new();
    let file_path = app.state.music_dir.join("right-version.mp3");
    let other_path = app.state.music_dir.join("ai.mp3");
    fs::write(&file_path, b"audio").unwrap();
    fs::write(&other_path, b"audio").unwrap();
    app.insert_track(
        TRACK_ID,
        "Song (Right version)",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    app.insert_track(
        OTHER_TRACK_ID,
        "Song (AI cover)",
        "Artist",
        other_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    app.set_tags(TRACK_ID, &["right-version"]);
    app.set_tags(OTHER_TRACK_ID, &["ai-cover"]);

    let right_version = app
        .request(
            Method::GET,
            "/api/v1/tracks?category=right-version",
            Body::empty(),
        )
        .await;
    assert_eq!(right_version.status(), StatusCode::OK);
    let body = body_json(right_version).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["id"], TRACK_ID);
    assert_eq!(body[0]["tags"][0]["name"], "right-version");

    let ai_cover = app
        .request(
            Method::GET,
            "/api/v1/tracks?category=ai-cover",
            Body::empty(),
        )
        .await;
    assert_eq!(ai_cover.status(), StatusCode::OK);
    let body = body_json(ai_cover).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["id"], OTHER_TRACK_ID);
}

#[tokio::test]
async fn patch_can_update_only_tags() {
    let app = TestApp::new();
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

    let response = app
        .json(
            Method::PATCH,
            &format!("/api/v1/tracks/{TRACK_ID}"),
            json!({"tags": ["right-version", "right-version", "ai-cover", "cover", "ost", "mashup"]}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    for _ in 0..20 {
        let count: i64 = app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM track_tag WHERE track_id = ?1",
                params![TRACK_ID],
                |row| row.get(0),
            )
            .unwrap();
        if count == 5 {
            assert!(mus_backend::db::track_tags_finalized(&app.state, TRACK_ID)
                .await
                .unwrap());
            {
                let mut conn = app.state.db.lock().unwrap();
                mus_backend::db::init_db(&mut conn).unwrap();
            }
            assert_eq!(
                mus_backend::db::get_track(&app.state, TRACK_ID)
                    .await
                    .unwrap()
                    .unwrap()
                    .tags
                    .len(),
                5
            );
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("tags were not updated");
}

#[tokio::test]
async fn patch_finalizes_an_explicit_empty_category_selection() {
    let app = TestApp::new();
    let file_path = app.state.music_dir.join("song.mp3");
    fs::write(&file_path, b"audio").unwrap();
    app.insert_track(
        TRACK_ID,
        "Song (cover)",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );

    let response = app
        .json(
            Method::PATCH,
            &format!("/api/v1/tracks/{TRACK_ID}"),
            json!({"tags": []}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !mus_backend::db::track_tags_finalized(&app.state, TRACK_ID)
            .await
            .unwrap()
        {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();

    {
        let mut conn = app.state.db.lock().unwrap();
        mus_backend::db::init_db(&mut conn).unwrap();
    }
    assert!(mus_backend::db::get_track(&app.state, TRACK_ID)
        .await
        .unwrap()
        .unwrap()
        .tags
        .is_empty());
}

#[tokio::test]
async fn patch_reinfers_tags_when_renaming_track() {
    let app = TestApp::new();
    let file_path = app.state.music_dir.join("song.wav");
    write_silent_wav(&file_path);
    app.insert_track(
        TRACK_ID,
        "Song (AI cover)",
        "Artist",
        file_path.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    app.set_tags(TRACK_ID, &["ai-cover"]);

    let response = app
        .json(
            Method::PATCH,
            &format!("/api/v1/tracks/{TRACK_ID}"),
            json!({"title": "Song (cover)", "artist": "Artist", "rename_file": true}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    for _ in 0..40 {
        let tags: Vec<String> = app
            .state
            .db
            .lock()
            .unwrap()
            .prepare(
                "SELECT tag.name FROM tag JOIN track_tag ON track_tag.tag_id = tag.id ORDER BY tag.name",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        if tags == ["cover"] {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("tags were not reinferred after rename");
}

#[tokio::test]
async fn editing_a_download_keeps_its_selected_categories() {
    let app = TestApp::new();
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
    assert!(
        mus_backend::db::set_download_tags(&app.state, TRACK_ID, &file_path, &["ost".into()])
            .await
            .unwrap()
    );
    let response = app
        .json(
            Method::PATCH,
            &format!("/api/v1/tracks/{TRACK_ID}"),
            json!({"title":"Song (cover)"}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    for _ in 0..40 {
        let track = mus_backend::db::list_tracks(&app.state)
            .await
            .unwrap()
            .pop()
            .unwrap();
        if track.title == "Song (cover)" {
            assert_eq!(
                track
                    .tags
                    .iter()
                    .map(|tag| tag.name.as_str())
                    .collect::<Vec<_>>(),
                ["ost"]
            );
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("track update did not finish");
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
async fn cut_preview_is_read_only_and_rejects_invalid_ranges() {
    let app = TestApp::new();
    let path = app.state.music_dir.join("song.wav");
    write_silent_wav(&path);
    let original = fs::read(&path).unwrap();
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        path.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    let response = app
        .request(
            Method::GET,
            &format!("/api/v1/tracks/{TRACK_ID}/cut-info"),
            Body::empty(),
        )
        .await;
    assert_eq!(body_json(response).await["duration_seconds"], 1.0);
    let response = app
        .json(
            Method::POST,
            &format!("/api/v1/tracks/{TRACK_ID}/cut-preview"),
            json!({"remove_section": {"start_seconds": 0.25, "end_seconds": 0.75}}),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "audio/wav");
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(fs::read(&path).unwrap(), original);
    for cut in [
        json!({"start_seconds": -1, "end_seconds": 0.5}),
        json!({"start_seconds": 0.5, "end_seconds": 0.25}),
        json!({"start_seconds": 0, "end_seconds": 2}),
        json!({"start_seconds": 0, "end_seconds": 1}),
    ] {
        for (method, uri, payload) in [
            (
                Method::POST,
                format!("/api/v1/tracks/{TRACK_ID}/cut-preview"),
                json!({"remove_section": cut}),
            ),
            (
                Method::PATCH,
                format!("/api/v1/tracks/{TRACK_ID}"),
                json!({"remove_section": cut}),
            ),
        ] {
            assert_eq!(
                app.json(method, &uri, payload).await.status(),
                StatusCode::BAD_REQUEST
            );
            assert_eq!(fs::read(&path).unwrap(), original);
        }
    }
}

#[tokio::test]
async fn cut_updates_duration_hash_and_events_without_reordering_the_track() {
    let app = TestApp::new();
    let path = app.state.music_dir.join("song.wav");
    write_silent_wav(&path);
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        path.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    app.set_tags(TRACK_ID, &["right-version"]);
    let mut events = app.state.events.subscribe();
    let response = app
        .json(
            Method::PATCH,
            &format!("/api/v1/tracks/{TRACK_ID}"),
            json!({"remove_section": {"start_seconds": 0.25, "end_seconds": 0.75}}),
        )
        .await;
    assert_eq!(body_json(response).await["status"], "queued");
    let event = tokio::time::timeout(std::time::Duration::from_secs(10), events.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        event.action_key.as_deref(),
        Some("track_updated"),
        "{event:?}"
    );
    let payload = event.action_payload.unwrap();
    assert_eq!(payload["previous_id"], TRACK_ID);
    assert_eq!(payload["tags"][0]["name"], "right-version");
    assert_ne!(payload["id"], TRACK_ID);
    assert_eq!(payload["duration"], 0); // Existing API durations are whole seconds.
    let new_id = payload["id"].as_str().unwrap();
    let conn = app.state.db.lock().unwrap();
    let added_at: i64 = conn
        .query_row(
            "SELECT added_at FROM track WHERE id = ?1",
            params![new_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(added_at, 1000);
    assert_eq!(
        mus_backend::files::file_content_hash(&path).unwrap(),
        new_id
    );
}

#[tokio::test]
async fn hls_playlist_rebuilds_missing_or_empty_media() {
    let app = TestApp::new();
    let source = app.state.music_dir.join("song.wav");
    write_silent_wav(&source);
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        source.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    let cache = app
        .state
        .data_dir
        .join(format!(".cache/hls/{TRACK_ID}/{TRACK_ID}"));
    fs::create_dir_all(&cache).unwrap();
    let url = format!("/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/index.m3u8");

    for incomplete_file in ["segment-00001.m4s", "segment-00000.m4s", "init.mp4"] {
        // A cache can lose a segment while another remains, or retain a truncated file.
        fs::write(cache.join("index.m3u8"), b"#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:6,\nsegment-00000.m4s\n#EXTINF:6,\nsegment-00001.m4s\n#EXT-X-ENDLIST\n").unwrap();
        fs::write(cache.join("init.mp4"), b"cached init").unwrap();
        fs::write(cache.join("segment-00000.m4s"), b"cached segment").unwrap();
        fs::write(cache.join("segment-00001.m4s"), b"cached segment").unwrap();
        if incomplete_file == "segment-00001.m4s" {
            fs::remove_file(cache.join(incomplete_file)).unwrap();
        } else {
            fs::write(cache.join(incomplete_file), []).unwrap();
        }

        let response = app.request(Method::GET, &url, Body::empty()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let playlist = body_text(response).await;
        for name in playlist
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let media = app
                .request(
                    Method::GET,
                    &format!("/api/v1/tracks/{TRACK_ID}/hls/{TRACK_ID}/{name}"),
                    Body::empty(),
                )
                .await;
            assert_eq!(media.status(), StatusCode::OK, "{name}");
            assert!(fs::metadata(cache.join(name)).unwrap().len() > 0);
        }
        assert!(fs::metadata(cache.join("init.mp4")).unwrap().len() > 0);
    }
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
async fn queued_edit_failure_preserves_the_track_and_allows_a_later_edit() {
    let app = TestApp::new();
    let original = app.state.music_dir.join("original.mp3");
    let destination = app.state.music_dir.join("Artist - Song.mp3");
    fs::write(&original, b"original audio").unwrap();
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        original.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    app.set_tags(TRACK_ID, &["cover"]);
    let before = body_json(
        app.request(Method::GET, "/api/v1/tracks", Body::empty())
            .await,
    )
    .await;
    let sse = app
        .request(Method::GET, "/api/v1/events/track-updates", Body::empty())
        .await;
    assert_eq!(sse.status(), StatusCode::OK);
    let mut stream = sse.into_body().into_data_stream();

    for expected_action in ["track_update_failed", "track_updated"] {
        let guard = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            app.state.mutation_lock(format!("track:{TRACK_ID}")),
        )
        .await
        .unwrap();
        let response = app
            .json(
                Method::PATCH,
                &format!("/api/v1/tracks/{TRACK_ID}"),
                json!({"rename_file": true, "tags": ["cover"]}),
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_json(response).await, json!({"status": "queued"}));
        if expected_action == "track_update_failed" {
            // Another writer takes the name after validation, before the worker runs.
            fs::write(&destination, b"competing audio").unwrap();
        }
        drop(guard);

        let chunk = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            futures_util::StreamExt::next(&mut stream),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        let text = std::str::from_utf8(&chunk).unwrap();
        let event: Value =
            serde_json::from_str(text.strip_prefix("data: ").unwrap().trim_end()).unwrap();
        assert_eq!(event["action_key"], expected_action, "{event}");
        assert_eq!(event["action_payload"]["id"], TRACK_ID);
        if expected_action == "track_update_failed" {
            assert_eq!(event["message_level"], "error");
            assert_eq!(
                event["action_payload"]["error"],
                "A file with this name already exists"
            );
            assert_eq!(fs::read(&original).unwrap(), b"original audio");
            assert_eq!(fs::read(&destination).unwrap(), b"competing audio");
            assert_eq!(
                body_json(
                    app.request(Method::GET, "/api/v1/tracks", Body::empty())
                        .await
                )
                .await,
                before
            );
            assert!(!mus_backend::db::track_tags_finalized(&app.state, TRACK_ID)
                .await
                .unwrap());
            fs::remove_file(&destination).unwrap();
        } else {
            assert_eq!(event["action_payload"]["previous_id"], TRACK_ID);
            assert!(!original.exists());
            assert_eq!(fs::read(&destination).unwrap(), b"original audio");
            let track = mus_backend::db::get_track(&app.state, TRACK_ID)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(std::path::Path::new(&track.file_path), destination);
        }
    }
}

#[tokio::test]
async fn deletion_waits_for_a_track_edit_and_uses_its_published_path() {
    let app = TestApp::new();
    let original = app.state.music_dir.join("original.mp3");
    let renamed = app.state.music_dir.join("renamed.mp3");
    fs::write(&original, b"original audio").unwrap();
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        original.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    let guard = app.state.mutation_lock(format!("track:{TRACK_ID}")).await;
    let uri = format!("/api/v1/tracks/{TRACK_ID}");
    let deletion = app.request(Method::DELETE, &uri, Body::empty());
    tokio::pin!(deletion);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut deletion)
            .await
            .is_err(),
        "deletion must not run while an edit owns the track"
    );
    fs::rename(&original, &renamed).unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE track SET file_path = ?1 WHERE id = ?2",
            params![renamed.to_str().unwrap(), TRACK_ID],
        )
        .unwrap();
    drop(guard);

    let response = deletion.await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(!original.exists());
    assert!(!renamed.exists());
    assert!(mus_backend::db::get_track(&app.state, TRACK_ID)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn deleting_a_track_whose_id_changes_during_an_edit_returns_not_found() {
    let app = TestApp::new();
    let path = app.state.music_dir.join("song.wav");
    write_silent_wav(&path);
    app.insert_track(
        TRACK_ID,
        "Song",
        "Artist",
        path.to_str().unwrap(),
        false,
        "COMPLETE",
    );
    let mut events = app.state.events.subscribe();
    let guard = app.state.mutation_lock(format!("track:{TRACK_ID}")).await;
    let uri = format!("/api/v1/tracks/{TRACK_ID}");
    let edit = app
        .json(
            Method::PATCH,
            &uri,
            json!({"remove_section": {"start_seconds": 0.25, "end_seconds": 0.75}}),
        )
        .await;
    assert_eq!(body_json(edit).await["status"], "queued");
    tokio::task::yield_now().await;
    let deletion = app.request(Method::DELETE, &uri, Body::empty());
    tokio::pin!(deletion);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut deletion)
            .await
            .is_err()
    );
    drop(guard);

    let response = tokio::time::timeout(std::time::Duration::from_secs(10), deletion)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body_json(response).await["detail"],
        "Track not found; refresh and try again"
    );
    let event = tokio::time::timeout(std::time::Duration::from_secs(10), events.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_updated"));
    let payload = event.action_payload.unwrap();
    let new_id = payload["id"].as_str().unwrap();
    assert_ne!(new_id, TRACK_ID);
    assert!(path.exists());
    let retry = app
        .request(
            Method::DELETE,
            &format!("/api/v1/tracks/{new_id}"),
            Body::empty(),
        )
        .await;
    assert_eq!(retry.status(), StatusCode::ACCEPTED);
    assert!(!path.exists());
}

#[tokio::test]
async fn rejected_uploads_remove_all_staged_files() {
    let long_title = "x".repeat(256);
    for fields in [
        vec![("artist", "Artist")],
        vec![("title", "Title")],
        vec![
            ("title", "Title"),
            ("artist", "Artist"),
            ("file", "invalid.txt"),
        ],
        vec![("title", long_title.as_str()), ("artist", "Artist")],
    ] {
        let app = TestApp::new();
        let boundary = "mus-upload-cleanup";
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"song.mp3\"\r\n\r\naudio\r\n"
        );
        for (name, value) in fields {
            if name == "file" {
                body.push_str(&format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{value}\"\r\n\r\ninvalid\r\n"
                ));
            } else {
                body.push_str(&format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                ));
            }
        }
        body.push_str(&format!("--{boundary}--\r\n"));
        let response = app
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
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let staged = fs::read_dir(app.state.data_dir.join(".cache"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        assert!(staged.is_empty(), "staged uploads leaked: {staged:?}");
        assert_eq!(fs::read_dir(&app.state.music_dir).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn upload_size_limit_checks_streamed_bytes_and_cleans_staging() {
    const CHILD: &str = "MUS_UPLOAD_SIZE_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        // Isolate the process-wide configuration from parallel API tests.
        let output = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "upload_size_limit_checks_streamed_bytes_and_cleans_staging",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("MAX_UPLOAD_SIZE_MB", "1")
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let limit = 1024 * 1024;
    for size in [limit - 1, limit, limit + 1] {
        let app = TestApp::new();
        let boundary = "mus-upload-limit";
        let mut chunks = vec![axum::body::Bytes::from(format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"song.mp3\"\r\n\r\n"
        ))];
        chunks.extend(
            vec![b'a'; size]
                .chunks(64 * 1024)
                .map(axum::body::Bytes::copy_from_slice),
        );
        chunks.push(axum::body::Bytes::from(format!("\r\n--{boundary}--\r\n")));
        let response = app
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
                    .body(Body::from_stream(futures_util::stream::iter(
                        chunks.into_iter().map(Ok::<_, std::io::Error>),
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, detail) = if size > limit {
            (
                StatusCode::PAYLOAD_TOO_LARGE,
                "File size exceeds maximum allowed size (1MB)",
            )
        } else {
            // Reaching field validation proves the complete file passed the size check.
            (StatusCode::BAD_REQUEST, "Missing title")
        };
        assert_eq!(response.status(), status, "file size: {size}");
        assert_eq!(body_json(response).await, json!({"detail": detail}));
        assert_eq!(fs::read_dir(&app.state.music_dir).unwrap().count(), 0);
        assert!(fs::read_dir(app.state.data_dir.join(".cache"))
            .unwrap()
            .all(|entry| !entry.unwrap().path().is_file()));
        assert!(mus_backend::db::list_tracks(&app.state)
            .await
            .unwrap()
            .is_empty());
    }
}

#[tokio::test]
async fn player_state_rejects_invalid_playback_values_without_changing_saved_state() {
    let app = TestApp::new();
    let uri = "/api/v1/player/state";
    let saved = body_json(app.request(Method::GET, uri, Body::empty()).await).await;
    for (field, value) in [
        ("volume_level", json!(-0.1)),
        ("volume_level", json!(1.1)),
        ("volume_level", json!(1e100)),
        ("progress_seconds", json!(-1)),
    ] {
        let mut invalid = saved.clone();
        invalid[field] = value;
        let response = app.json(Method::POST, uri, invalid).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(
            body_json(app.request(Method::GET, uri, Body::empty()).await).await,
            saved,
        );
    }
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
            "is_playing": false,
            "is_shuffle": false,
            "is_repeat": false
        })
    );

    let payload = json!({
        "current_track_id": TRACK_ID,
        "progress_seconds": 45.5,
        "volume_level": 0.7,
        "is_muted": true,
        "is_playing": true,
        "is_shuffle": false,
        "is_repeat": true
    });
    let saved = app
        .json(Method::POST, "/api/v1/player/state", payload.clone())
        .await;
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(body_json(saved).await, payload);

    let loaded = app
        .request(Method::GET, "/api/v1/player/state", Body::empty())
        .await;
    assert_eq!(loaded.status(), StatusCode::OK);
    assert_eq!(body_json(loaded).await, payload);

    let updated_payload = json!({
        "current_track_id": OTHER_TRACK_ID,
        "progress_seconds": 12.25,
        "volume_level": 0.4,
        "is_muted": false,
        "is_playing": false,
        "is_shuffle": true,
        "is_repeat": false
    });
    let updated = app
        .json(
            Method::POST,
            "/api/v1/player/state",
            updated_payload.clone(),
        )
        .await;
    assert_eq!(updated.status(), StatusCode::OK);

    let loaded_updated = app
        .request(Method::GET, "/api/v1/player/state", Body::empty())
        .await;
    assert_eq!(loaded_updated.status(), StatusCode::OK);
    assert_eq!(body_json(loaded_updated).await, updated_payload);

    let row_count: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM playerstate", [], |row| row.get(0))
        .unwrap();
    assert_eq!(row_count, 1);
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
    let permit = app.state.download_lock.try_acquire().unwrap();
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
    drop(permit);

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

    let event = json!({
        "message_to_show": "Песня\nupdated",
        "message_level": "info",
        "action_key": "track_updated",
        "action_payload": {"id": TRACK_ID, "title": "Песня"}
    });
    let triggered = app
        .json(Method::POST, "/api/v1/events/trigger", event.clone())
        .await;
    assert_eq!(triggered.status(), StatusCode::OK);
    let mut stream = sse.into_body().into_data_stream();
    let chunk = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        futures_util::StreamExt::next(&mut stream),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    let text = std::str::from_utf8(&chunk).unwrap();
    let data = text.strip_prefix("data: ").unwrap().trim_end();
    assert_eq!(serde_json::from_str::<Value>(data).unwrap(), event);

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
