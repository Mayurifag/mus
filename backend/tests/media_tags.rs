use std::time::SystemTime;
use std::{fs, path::Path, process::Command};

use id3::{
    frame::{Picture, PictureType},
    Tag, TagLike, Version,
};
use mus_backend::media::{extract_cover, standardize_audio_tags, write_audio_tags};
use serde_json::Value;
use tempfile::TempDir;

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn probe(path: &Path) -> Value {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn create_mp3_with_cover(path: &Path) {
    run(Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.1",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=32x32:d=0.1",
            "-map",
            "0:a",
            "-map",
            "1:v",
            "-c:a",
            "libmp3lame",
            "-q:a",
            "9",
            "-c:v",
            "mjpeg",
            "-disposition:v",
            "attached_pic",
            "-metadata",
            "title=Old title",
            "-metadata",
            "artist=Old artist",
            "-id3v2_version",
            "4",
        ])
        .arg(path));
}

fn create_wav_with_id3v24(path: &Path) {
    run(Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(path));
    let mut tag = Tag::new();
    tag.set_title("Old title");
    tag.set_artist("Old artist");
    tag.write_to_path(path, Version::Id3v24).unwrap();
}

fn create_flac_with_cover(path: &Path) {
    run(Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.1",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=32x32:d=0.1",
            "-map",
            "0:a",
            "-map",
            "1:v",
            "-c:a",
            "flac",
            "-c:v",
            "mjpeg",
            "-disposition:v",
            "attached_pic",
            "-metadata",
            "title=FLAC title",
            "-metadata",
            "artist=FLAC artist",
        ])
        .arg(path));
}

fn create_jpeg(path: &Path) {
    run(Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=green:s=32x32:d=0.1",
            "-frames:v",
            "1",
            "-c:v",
            "mjpeg",
        ])
        .arg(path));
}

fn add_id3_cover(path: &Path, cover_path: &Path) {
    let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());
    tag.add_frame(Picture {
        mime_type: "image/jpeg".to_string(),
        picture_type: PictureType::CoverFront,
        description: String::new(),
        data: fs::read(cover_path).unwrap(),
    });
    tag.write_to_path(path, Version::Id3v23).unwrap();
}

fn assert_id3v23(path: &Path) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[0..3], b"ID3");
    assert_eq!(bytes[3], 3);
}

fn assert_has_attached_cover(data: &Value) {
    let has_cover = data["streams"].as_array().unwrap().iter().any(|stream| {
        stream["codec_type"] == "video" && stream["disposition"]["attached_pic"] == 1
    });
    assert!(has_cover);
}

fn assert_no_mus_files(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let file_name = entry.unwrap().file_name();
        assert!(!file_name.to_string_lossy().starts_with(".mus-"));
    }
}

#[tokio::test]
async fn ffmpeg_tag_rewrite_preserves_mp3_id3v23_utf8_and_cover() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.mp3");
    create_mp3_with_cover(&path);

    standardize_audio_tags(&path).await.unwrap();
    assert_id3v23(&path);
    assert_has_attached_cover(&probe(&path));

    write_audio_tags(
        &path,
        &tmp.path().join("cache"),
        "Тестовый трек",
        "アーティスト",
    )
    .await
    .unwrap();
    assert_id3v23(&path);
    let data = probe(&path);
    assert_eq!(data["format"]["tags"]["title"], "Тестовый трек");
    assert_eq!(data["format"]["tags"]["artist"], "アーティスト");
    assert_has_attached_cover(&data);
}

#[tokio::test]
async fn id3_tag_rewrite_standardizes_wav_to_id3v23() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.wav");
    create_wav_with_id3v24(&path);

    standardize_audio_tags(&path).await.unwrap();
    let tag = Tag::read_from_path(&path).unwrap();
    assert_eq!(tag.version(), Version::Id3v23);

    write_audio_tags(
        &path,
        &tmp.path().join("cache"),
        "Тестовый трек",
        "アーティスト",
    )
    .await
    .unwrap();
    let tag = Tag::read_from_path(&path).unwrap();
    assert_eq!(tag.version(), Version::Id3v23);
    assert_eq!(tag.title(), Some("Тестовый трек"));
    assert_eq!(tag.artist(), Some("アーティスト"));
    assert!(probe(&path)["format"]["duration"].as_str().is_some());
}

#[tokio::test]
async fn standardize_audio_tags_skips_already_standard_id3_files() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.wav");
    create_wav_with_id3v24(&path);
    standardize_audio_tags(&path).await.unwrap();
    let before = fs::metadata(&path).unwrap().modified().unwrap();

    standardize_audio_tags(&path).await.unwrap();
    let after = fs::metadata(&path).unwrap().modified().unwrap();

    assert_eq!(before, after);
}

#[tokio::test]
async fn standardize_audio_tags_skips_non_id3_formats() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.flac");
    run(Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.1",
            "-c:a",
            "flac",
        ])
        .arg(&path));
    let original = SystemTime::UNIX_EPOCH;
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(original)).unwrap();

    standardize_audio_tags(&path).await.unwrap();

    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), original);
}

#[tokio::test]
async fn ffmpeg_tag_rewrite_uses_cache_dir_with_long_filename() {
    let tmp = TempDir::new().unwrap();
    let music_dir = tmp.path().join("music");
    let cache_dir = tmp.path().join("cache");
    fs::create_dir(&music_dir).unwrap();
    let path = music_dir.join(format!("{}.flac", "a".repeat(250)));
    create_flac_with_cover(&path);

    write_audio_tags(&path, &cache_dir, "New title", "New artist")
        .await
        .unwrap();

    let data = probe(&path);
    assert_eq!(data["format"]["tags"]["title"], "New title");
    assert_eq!(data["format"]["tags"]["artist"], "New artist");
    assert_has_attached_cover(&data);
    assert_no_mus_files(&music_dir);
    assert!(fs::read_dir(&cache_dir).unwrap().next().is_none());
}

#[tokio::test]
async fn ffmpeg_tag_rewrite_cleans_cache_after_failure() {
    let tmp = TempDir::new().unwrap();
    let music_dir = tmp.path().join("music");
    let cache_dir = tmp.path().join("cache");
    fs::create_dir(&music_dir).unwrap();
    let path = music_dir.join("song.flac");
    fs::write(&path, b"not audio").unwrap();

    assert!(
        write_audio_tags(&path, &cache_dir, "New title", "New artist")
            .await
            .is_err()
    );

    assert_no_mus_files(&music_dir);
    assert!(fs::read_dir(&cache_dir).unwrap().next().is_none());
}

#[tokio::test]
async fn extract_cover_supports_flac_embedded_cover() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.flac");
    let covers_dir = tmp.path().join("covers");
    fs::create_dir(&covers_dir).unwrap();
    create_flac_with_cover(&path);

    assert!(extract_cover(&path, &covers_dir, 1).await.unwrap());
    assert!(covers_dir.join("1_original.webp").is_file());
    assert!(covers_dir.join("1_small.webp").is_file());
}

#[tokio::test]
async fn extract_cover_supports_wav_id3_cover() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.wav");
    let covers_dir = tmp.path().join("covers");
    let cover_path = tmp.path().join("cover.jpg");
    fs::create_dir(&covers_dir).unwrap();
    create_wav_with_id3v24(&path);
    create_jpeg(&cover_path);
    add_id3_cover(&path, &cover_path);

    assert!(extract_cover(&path, &covers_dir, 1).await.unwrap());
    assert!(covers_dir.join("1_original.webp").is_file());
    assert!(covers_dir.join("1_small.webp").is_file());
}
