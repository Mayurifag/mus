use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    http::{Response, StatusCode},
    routing::get,
    Router,
};
use id3::{
    frame::{Chapter, Picture, PictureType},
    Tag, TagLike, Version,
};
use reqwest::{Client, Request};
use serde_json::json;
use tempfile::TempDir;
use tokio::{process::Command, task::JoinHandle};

use super::{
    info::{self, SourceMetadata},
    metadata_response, publish,
};

fn fallback_command(tmp: &TempDir) -> Command {
    use std::os::unix::fs::PermissionsExt;
    let path = tmp.path().join("yt-dlp");
    fs::write(&path, "#!/bin/sh\ncase \"$1\" in\n--version) printf '%s\\n' '2026.10.19.1' ;;\n--dump-json) printf '%s\\n' '{\"title\":\"Fallback Song\",\"artist\":\"Fallback Artist\",\"duration\":123}' ;;\n*) exit 1 ;;\nesac\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let mut command = Command::new(path);
    command.args(
        info::metadata_command("https://example.com/video")
            .as_std()
            .get_args(),
    );
    command
}

async fn oembed_server(
    status: StatusCode,
    body: String,
    delay: Duration,
) -> (Client, Request, JoinHandle<()>, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let router = Router::new().route(
        "/",
        get(move || {
            let body = body.clone();
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(delay).await;
                Response::builder()
                    .status(status)
                    .header("location", "/")
                    .body(Body::from(body))
                    .unwrap()
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = info::oembed_client().no_proxy().build().unwrap();
    let mut request =
        info::oembed_request(&client, "https://youtu.be/dQw4w9WgXcQ?token=private-token").unwrap();
    let query = request.url().query().unwrap();
    let url = format!("http://{addr}/?{query}").parse().unwrap();
    *request.url_mut() = url;
    (client, request, server, calls)
}

fn oembed_body() -> String {
    json!({
        "title": "Artist - Song Official Video",
        "author_name": "Publisher",
        "thumbnail_url": "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg",
        "html": "<iframe>never use untrusted embed HTML</iframe>"
    })
    .to_string()
}

fn metadata_output(data: serde_json::Value) -> Command {
    let mut command = Command::new("printf");
    command.arg("%s").arg(data.to_string());
    command
}

#[test]
fn oembed_uses_only_fixed_host_and_canonical_validated_id() {
    let client = info::oembed_client().build().unwrap();
    let request = info::oembed_request(&client, "https://user:private-password@music.youtube.com/watch?v=dQw4w9WgXcQ&token=private-token#private-fragment").unwrap();
    assert_eq!(request.url().scheme(), "https");
    assert_eq!(request.url().host_str(), Some("www.youtube.com"));
    assert_eq!(request.url().path(), "/oembed");
    let params: Vec<_> = request.url().query_pairs().collect();
    assert_eq!(
        params,
        [
            (
                "url".into(),
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into()
            ),
            ("format".into(), "json".into())
        ]
    );
    assert!(!request.url().as_str().contains("private-"));
    for url in [
        "https://example.com/watch?v=dQw4w9WgXcQ",
        "https://youtube.com.evil.example/watch?v=dQw4w9WgXcQ",
        "https://youtube.com/watch?v=bad",
        "--exec=bad",
    ] {
        assert!(info::oembed_request(&client, url).is_none());
    }
}

#[tokio::test]
async fn fast_oembed_never_starts_ytdlp_and_returns_nullable_duration() {
    let (client, request, server, calls) =
        oembed_server(StatusCode::OK, oembed_body(), Duration::from_millis(30)).await;
    let started = Instant::now();
    let source =
        info::metadata_from_sources(Some((client, request)), Command::new("does-not-exist"))
            .await
            .unwrap();
    let response = metadata_response(&source, &[]);
    server.abort();
    let _ = server.await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(response.title, "Song");
    assert_eq!(response.artist, "Artist");
    assert!(response.duration.is_none());
    assert!(response
        .thumbnail_url
        .unwrap()
        .starts_with("https://i.ytimg.com/"));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn ambiguous_oembed_uses_structured_credits_without_losing_version() {
    let body = json!({"title":"Song (Acoustic)", "author_name":"Artist - Topic"}).to_string();
    let (client, request, server, _) = oembed_server(StatusCode::OK, body, Duration::ZERO).await;
    let extracted = json!({
        "title":"Song (Acoustic)", "artist":"ARTIST, Guest", "artists":["ARTIST", "Guest"], "channel":"artistworld", "duration":180
    });
    let source = info::metadata_from_sources(Some((client, request)), metadata_output(extracted))
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
    let response = metadata_response(&source, &[]);
    assert_eq!(response.title, "Song (Acoustic)");
    assert_eq!(response.artist, "Artist; Guest");
    assert_eq!(response.duration, Some(180.0));
}

#[tokio::test]
async fn structured_artist_list_preserves_commas_in_names() {
    let body = json!({"title":"Song", "author_name":"Tyler, The Creator - Topic"}).to_string();
    let (client, request, server, _) = oembed_server(StatusCode::OK, body, Duration::ZERO).await;
    let extracted = json!({
        "title":"Song", "artist":"Tyler, The Creator, Guest",
        "artists":["Tyler, The Creator", "Guest"], "duration":180
    });
    let source = info::metadata_from_sources(Some((client, request)), metadata_output(extracted))
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
    let response = metadata_response(&source, &[]);
    assert_eq!(response.artist, "Tyler, The Creator; Guest");
    assert_eq!(response.title, "Song");
}

#[tokio::test]
async fn extractor_keeps_source_artist_credits_without_guessing_comma_boundaries() {
    for (artists, raw, expected) in [
        (
            json!(["Tyler, The Creator", "Guest"]),
            json!(null),
            Some("Tyler, The Creator; Guest"),
        ),
        (
            json!(["Tyler, The Creator", "Guest"]),
            json!("Tyler, The Creator,Guest"),
            Some("Tyler, The Creator; Guest"),
        ),
        (
            json!(["Artist", "Guest"]),
            json!("Artist"),
            Some("Artist; Guest"),
        ),
        (
            json!(["Artist", "Guest"]),
            json!("Other"),
            Some("Artist; Guest; Other"),
        ),
        (
            json!(["Artist", null, 2, "Guest"]),
            json!(null),
            Some("Artist; Guest"),
        ),
        (json!([]), json!("Artist, Guest"), Some("Artist, Guest")),
        (
            json!("malformed"),
            json!("Tyler, The Creator"),
            Some("Tyler, The Creator"),
        ),
        (json!(null), json!(null), None),
    ] {
        let data = json!({"title":"Song", "artists":artists, "artist":raw});
        let source = info::metadata_from_sources(None, metadata_output(data))
            .await
            .unwrap();
        assert_eq!(source.artist.as_deref(), expected);
    }
}

#[tokio::test]
async fn topic_is_not_duplicated_in_an_ambiguous_comma_credit() {
    for (topic, artist, expected) in [
        ("Artist - Topic", "Artist, Guest", "Artist, Guest"),
        (
            "Tyler, The Creator - Topic",
            "Tyler, The Creator, Guest",
            "Tyler, The Creator, Guest",
        ),
        ("Artist - Topic", "Other, Guest", "Artist; Other, Guest"),
    ] {
        let body = json!({"title":"Song", "author_name":topic}).to_string();
        let (client, request, server, _) =
            oembed_server(StatusCode::OK, body, Duration::ZERO).await;
        let source = info::metadata_from_sources(
            Some((client, request)),
            metadata_output(json!({"title":"Song", "artist":artist})),
        )
        .await
        .unwrap();
        server.abort();
        let _ = server.await;
        assert_eq!(metadata_response(&source, &[]).artist, expected);
    }
}

#[tokio::test]
async fn optional_extraction_failure_keeps_oembed_metadata() {
    let body = json!({"title":"Song (Acoustic)", "author_name":"Artist - Topic"}).to_string();
    let (client, request, server, _) = oembed_server(StatusCode::OK, body, Duration::ZERO).await;
    let source =
        info::metadata_from_sources(Some((client, request)), Command::new("does-not-exist"))
            .await
            .unwrap();
    server.abort();
    let _ = server.await;
    let response = metadata_response(&source, &[]);
    assert_eq!(response.title, "Song (Acoustic)");
    assert_eq!(response.artist, "Artist");
    assert_eq!(response.duration, None);
}

#[tokio::test]
async fn conflicting_source_title_does_not_replace_oembed_version() {
    let body = json!({"title":"Song (Acoustic)", "author_name":"Artist - Topic"}).to_string();
    let (client, request, server, _) = oembed_server(StatusCode::OK, body, Duration::ZERO).await;
    let extracted = json!({"title":"Song (Live)", "artist":"Other", "duration":180});
    let source = info::metadata_from_sources(Some((client, request)), metadata_output(extracted))
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
    let response = metadata_response(&source, &[]);
    assert_eq!(response.title, "Song (Acoustic)");
    assert_eq!(response.artist, "Artist");
    assert_eq!(response.duration, None);
}

#[test]
fn conflicting_topic_and_structured_credits_remain_editable() {
    let oembed = SourceMetadata {
        title: "Song".into(),
        artist: None,
        channel: "Artist - Topic".into(),
        thumbnail_url: None,
        duration: None,
    };
    let extracted = SourceMetadata {
        title: "Song".into(),
        artist: Some("Other".into()),
        channel: "Publisher".into(),
        thumbnail_url: None,
        duration: Some(180.0),
    };
    let response = metadata_response(&info::merge_metadata(oembed, extracted), &[]);
    assert_eq!(response.artist, "Artist; Other");
    assert_eq!(response.duration, Some(180.0));
}

#[tokio::test]
async fn unavailable_invalid_redirected_or_oversized_oembed_uses_fallback_once() {
    for (status, body) in [
        (StatusCode::FORBIDDEN, "private-upstream-error".into()),
        (StatusCode::FOUND, oembed_body()),
        (StatusCode::OK, "private-not-json".into()),
        (StatusCode::OK, "{}".into()),
        (
            StatusCode::OK,
            json!({"title":"", "author_name":"Publisher"}).to_string(),
        ),
        (StatusCode::OK, "x".repeat(65 * 1024)),
    ] {
        let tmp = TempDir::new().unwrap();
        let (client, request, server, calls) = oembed_server(status, body, Duration::ZERO).await;
        let data = info::metadata_from_sources(Some((client, request)), fallback_command(&tmp))
            .await
            .unwrap();
        server.abort();
        let _ = server.await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(data.title, "Fallback Song");
        assert_eq!(data.duration, Some(123.0));
    }
    let tmp = TempDir::new().unwrap();
    let data = info::metadata_from_sources(None, fallback_command(&tmp))
        .await
        .unwrap();
    assert_eq!(data.artist.as_deref(), Some("Fallback Artist"));
}

#[tokio::test]
async fn slow_oembed_is_cut_off_at_two_seconds_then_uses_fallback() {
    let tmp = TempDir::new().unwrap();
    let (client, request, server, calls) =
        oembed_server(StatusCode::OK, oembed_body(), Duration::from_secs(20)).await;
    let started = Instant::now();
    let data = info::metadata_from_sources(Some((client, request)), fallback_command(&tmp))
        .await
        .unwrap();
    let elapsed = started.elapsed();
    server.abort();
    let _ = server.await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(data.title, "Fallback Song");
    assert!(elapsed >= Duration::from_millis(1900) && elapsed < Duration::from_secs(4));
}

#[test]
fn local_metadata_parsing_preserves_inferred_tags() {
    let data = SourceMetadata {
        title: "Artist - Song (AI cover)".into(),
        artist: None,
        channel: "Publisher".into(),
        thumbnail_url: None,
        duration: None,
    };
    let response = metadata_response(&data, &[]);
    assert_eq!(response.title, "Song (AI cover)");
    assert_eq!(response.artist, "Artist");
    assert_eq!(
        response
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["ai-cover"]
    );

    let data = SourceMetadata {
        title: "Song Official Audio".into(),
        artist: Some("Artist, Guest".into()),
        channel: "Publisher".into(),
        thumbnail_url: None,
        duration: Some(123.0),
    };
    let response = metadata_response(&data, &[]);
    assert_eq!(response.title, "Song");
    assert_eq!(response.artist, "Artist, Guest");
    assert_eq!(response.duration, Some(123.0));
    assert!(response.tags.is_empty());
}

#[test]
fn acoustic_metadata_uses_topic_or_structured_artist() {
    for (channel, artist) in [("Artist - Topic", None), ("artistworld", Some("Artist"))] {
        let data = SourceMetadata {
            title: "Song (Acoustic)".into(),
            artist: artist.map(str::to_string),
            channel: channel.into(),
            thumbnail_url: None,
            duration: Some(180.0),
        };
        let response = metadata_response(&data, &[]);
        assert_eq!(response.title, "Song (Acoustic)");
        assert_eq!(response.artist, "Artist");
    }
}

#[test]
fn metadata_uses_title_artist_when_uploader_is_unrelated() {
    let data = SourceMetadata {
        title: "Artist  - Song".into(),
        artist: None,
        channel: "Different Uploader".into(),
        thumbnail_url: None,
        duration: None,
    };
    let response = metadata_response(&data, &[]);
    assert_eq!(response.title, "Song");
    assert_eq!(response.artist, "Artist");
}

#[test]
fn metadata_uses_local_artist_spellings_without_changing_the_library() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().to_path_buf(),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    {
        let conn = state.db.lock().unwrap();
        conn.execute("INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,content_hash) VALUES (?1,'Old','Artist',1,'/old/song.mp3',1,1,?1)", rusqlite::params!["1".repeat(64)]).unwrap();
    }
    let data = SourceMetadata {
        title: "Song".into(),
        artist: None,
        channel: "ARTIST - Topic".into(),
        thumbnail_url: None,
        duration: None,
    };
    let response = metadata_response(&data, &crate::db::artist_values(&state).unwrap());
    assert_eq!(response.artist, "Artist");
    assert_eq!(response.title, "Song");
    assert_eq!(crate::db::list_tracks(&state).unwrap()[0].artist, "Artist");
}

#[tokio::test]
async fn stalled_metadata_extractor_has_an_eight_second_total_deadline() {
    let mut command = Command::new("sleep");
    command.arg("20");
    let started = Instant::now();
    let result = info::metadata_from_sources(None, command).await;
    let elapsed = started.elapsed();
    assert!(result.is_err());
    assert!(elapsed >= Duration::from_millis(7900) && elapsed < Duration::from_secs(11));
}

#[test]
fn metadata_extraction_and_download_flags_keep_quality_and_required_features() {
    let command = info::metadata_command("https://example.com?token=private-token");
    let args: Vec<_> = command
        .as_std()
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect();
    assert!(args
        .windows(2)
        .any(|args| args == ["--socket-timeout", "3"]));
    assert!(args
        .windows(2)
        .any(|args| args == ["--extractor-retries", "0"]));
    assert!(args.contains(&"--skip-download"));
    assert!(args.contains(&"--no-check-formats"));
    assert_eq!(args[args.len() - 2], "--");
    for embed_thumbnail in [true, false] {
        let command = super::download_command(
            "https://example.com",
            Path::new("/private-cache"),
            embed_thumbnail,
        );
        let args: Vec<_> = command
            .as_std()
            .get_args()
            .map(|arg| arg.to_str().unwrap())
            .collect();
        assert!(args.windows(2).any(|args| args == ["--audio-quality", "0"]));
        assert!(args
            .windows(2)
            .any(|args| args == ["--format", "bestaudio/best"]));
        assert!(args
            .windows(2)
            .any(|args| args == ["--sponsorblock-remove", "all"]));
        assert!(args.contains(&"--embed-chapters"));
        assert!(args.contains(&"--embed-metadata"));
        assert_eq!(args.contains(&"--embed-thumbnail"), embed_thumbnail);
    }
}

fn tagged_mp3(path: &Path, version: Version) -> Vec<u8> {
    let audio = vec![0x55; 1024 * 1024];
    fs::write(path, &audio).unwrap();
    let mut tag = Tag::new();
    tag.set_title("Old title");
    tag.set_artist("Old artist");
    tag.set_album("Preserved album");
    tag.add_frame(Chapter {
        element_id: "chapter-1".into(),
        start_time: 0,
        end_time: 1000,
        start_offset: u32::MAX,
        end_offset: u32::MAX,
        frames: Vec::new(),
    });
    tag.add_frame(Picture {
        mime_type: "image/jpeg".into(),
        picture_type: PictureType::CoverFront,
        description: "Existing cover".into(),
        data: vec![1, 2, 3],
    });
    tag.write_to_path(path, version).unwrap();
    audio
}

#[tokio::test]
async fn cached_tag_edit_and_single_publication_preserve_audio_chapters_and_artwork() {
    use std::os::unix::fs::MetadataExt;
    let tmp = TempDir::new().unwrap();
    let cache = tmp.path().join("cache");
    let music = tmp.path().join("music");
    fs::create_dir(&cache).unwrap();
    fs::create_dir(&music).unwrap();
    let staged = cache.join("download.mp3");
    let final_path = music.join("Artist - Song.mp3");
    let audio = tagged_mp3(&staged, Version::Id3v24);
    let original = Tag::read_from_path(&staged).unwrap();
    let inode = fs::metadata(&staged).unwrap().ino();
    publish::publish_download(
        &staged,
        &cache,
        &final_path,
        Some("Song"),
        Some("Artist"),
        None,
    )
    .await
    .unwrap();
    let tag = Tag::read_from_path(&final_path).unwrap();
    assert_eq!(tag.version(), Version::Id3v23);
    assert_eq!(tag.title(), Some("Song"));
    assert_eq!(tag.artist(), Some("Artist"));
    assert_eq!(tag.album(), Some("Preserved album"));
    assert_eq!(tag.get("CHAP"), original.get("CHAP"));
    assert_eq!(tag.get("APIC"), original.get("APIC"));
    assert!(fs::read(&final_path).unwrap().ends_with(&audio));
    assert_eq!(fs::metadata(&final_path).unwrap().ino(), inode);
    assert!(!staged.exists());
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
}

#[test]
fn matching_v23_tags_are_not_rewritten_and_custom_cover_preserves_chapters() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("download.mp3");
    let audio = tagged_mp3(&path, Version::Id3v23);
    let before = fs::read(&path).unwrap();
    assert!(!publish::prepare_tags(&path, Some("Old title"), Some("Old artist"), None).unwrap());
    assert_eq!(fs::read(&path).unwrap(), before);
    let chapter = Tag::read_from_path(&path).unwrap().get("CHAP").cloned();
    let cover = tmp.path().join("cover.jpg");
    fs::write(&cover, [4, 5, 6]).unwrap();
    assert!(publish::prepare_tags(
        &path,
        Some("Chosen title"),
        Some("Chosen artist"),
        Some(&cover)
    )
    .unwrap());
    let tag = Tag::read_from_path(&path).unwrap();
    assert_eq!(tag.get("CHAP"), chapter.as_ref());
    assert_eq!(tag.title(), Some("Chosen title"));
    assert_eq!(tag.artist(), Some("Chosen artist"));
    let mut expected = Tag::new();
    expected.add_frame(Picture {
        mime_type: "image/jpeg".into(),
        picture_type: PictureType::CoverFront,
        description: String::new(),
        data: vec![4, 5, 6],
    });
    assert_eq!(tag.get("APIC"), expected.get("APIC"));
    assert!(fs::read(&path).unwrap().ends_with(&audio));
}

#[tokio::test]
async fn publication_never_overwrites_an_existing_music_file() {
    let tmp = TempDir::new().unwrap();
    let staged = tmp.path().join("download.mp3");
    let final_path = tmp.path().join("existing.mp3");
    tagged_mp3(&staged, Version::Id3v23);
    fs::write(&final_path, b"existing music must be preserved").unwrap();
    let result = publish::publish_download(
        &staged,
        tmp.path(),
        &final_path,
        Some("Song"),
        Some("Artist"),
        None,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        fs::read(final_path).unwrap(),
        b"existing music must be preserved"
    );
    assert!(staged.exists());
}
