use std::{fs, path::Path, process::Command};

use id3::{Tag, TagLike, Version};
use mus_backend::media::{standardize_audio_tags, write_audio_tags};
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
            "sine=frequency=1000:duration=1",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=32x32:d=1",
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
            "sine=frequency=1000:duration=1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(path));
    let mut tag = Tag::new();
    tag.set_title("Old title");
    tag.set_artist("Old artist");
    tag.write_to_path(path, Version::Id3v24).unwrap();
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

#[tokio::test]
async fn ffmpeg_tag_rewrite_preserves_mp3_id3v23_utf8_and_cover() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.mp3");
    create_mp3_with_cover(&path);

    standardize_audio_tags(&path).await.unwrap();
    assert_id3v23(&path);
    assert_has_attached_cover(&probe(&path));

    write_audio_tags(&path, "Тестовый трек", "アーティスト")
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

    write_audio_tags(&path, "Тестовый трек", "アーティスト")
        .await
        .unwrap();
    let tag = Tag::read_from_path(&path).unwrap();
    assert_eq!(tag.version(), Version::Id3v23);
    assert_eq!(tag.title(), Some("Тестовый трек"));
    assert_eq!(tag.artist(), Some("アーティスト"));
    assert!(probe(&path)["format"]["duration"].as_str().is_some());
}
