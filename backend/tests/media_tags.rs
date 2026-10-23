use std::time::SystemTime;
use std::{fs, path::Path, process::Command};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use id3::{
    frame::{Picture, PictureType},
    Tag, TagLike, Version,
};
use mus_backend::media::{
    apply_audio_update, extract_cover, preview_cut, read_cut_source, standardize_audio_tags,
    write_audio_tags, AudioUpdate,
};
use mus_backend::models::AudioCut;
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

#[cfg(unix)]
fn file_id(path: &Path) -> u64 {
    fs::metadata(path).unwrap().ino()
}

#[tokio::test]
async fn ffmpeg_tag_rewrite_preserves_mp3_id3v23_utf8_and_cover() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.mp3");
    create_mp3_with_cover(&path);

    standardize_audio_tags(&path, &tmp.path().join("cache"))
        .await
        .unwrap();
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
async fn combined_audio_update_publishes_tags_cover_rename_and_mtime() {
    let tmp = TempDir::new().unwrap();
    let music_dir = tmp.path().join("music");
    let cache_dir = tmp.path().join("cache");
    fs::create_dir(&music_dir).unwrap();
    let path = music_dir.join("song.flac");
    let cover_path = tmp.path().join("cover.jpg");
    let target_path = music_dir.join("New artist - New title.flac");
    create_flac_with_cover(&path);
    create_jpeg(&cover_path);
    #[cfg(unix)]
    let original_file_id = file_id(&path);
    let original = SystemTime::UNIX_EPOCH;
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(original)).unwrap();

    let result = apply_audio_update(
        &path,
        &cache_dir,
        AudioUpdate {
            remove_section: None,
            title: Some("New title"),
            artist: Some("New artist"),
            cover_jpeg_path: Some(&cover_path),
            target_path: Some(&target_path),
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await
    .unwrap();

    assert!(result.changed);
    assert_eq!(result.path, target_path);
    assert!(!path.exists());
    assert!(target_path.is_file());
    #[cfg(unix)]
    assert_eq!(file_id(&target_path), original_file_id);
    assert!(fs::metadata(&target_path).unwrap().modified().unwrap() > original);
    let data = probe(&target_path);
    assert_eq!(data["format"]["tags"]["title"], "New title");
    assert_eq!(data["format"]["tags"]["artist"], "New artist");
    assert_has_attached_cover(&data);
    assert_no_mus_files(&music_dir);
    assert!(fs::read_dir(&cache_dir).unwrap().next().is_none());
}

#[tokio::test]
async fn combined_audio_update_normalizes_rename_and_tag_text() {
    let tmp = TempDir::new().unwrap();
    let music_dir = tmp.path().join("music");
    let cache_dir = tmp.path().join("cache");
    fs::create_dir(&music_dir).unwrap();
    let path = music_dir.join("song.mp3");
    let target_path = music_dir.join("LIL KRYSTALLL - Бедный Русский.mp3");
    create_mp3_with_cover(&path);

    let result = apply_audio_update(
        &path,
        &cache_dir,
        AudioUpdate {
            remove_section: None,
            title: Some("Бедныи\u{0306} Русскии\u{0306}"),
            artist: Some("LIL KRYSTALLL"),
            cover_jpeg_path: None,
            target_path: Some(&target_path),
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.path, target_path);
    assert!(!path.exists());
    assert!(target_path.is_file());
    let data = probe(&target_path);
    assert_eq!(data["format"]["tags"]["title"], "Бедный Русский");
    assert_eq!(data["format"]["tags"]["artist"], "LIL KRYSTALLL");
}

#[tokio::test]
async fn ffmpeg_tag_rewrite_preserves_existing_file_identity() {
    let tmp = TempDir::new().unwrap();
    let music_dir = tmp.path().join("music");
    let cache_dir = tmp.path().join("cache");
    fs::create_dir(&music_dir).unwrap();
    let path = music_dir.join("song.flac");
    create_flac_with_cover(&path);
    #[cfg(unix)]
    let original_file_id = file_id(&path);

    write_audio_tags(&path, &cache_dir, "New title", "New artist")
        .await
        .unwrap();

    #[cfg(unix)]
    assert_eq!(file_id(&path), original_file_id);
    let data = probe(&path);
    assert_eq!(data["format"]["tags"]["title"], "New title");
    assert_eq!(data["format"]["tags"]["artist"], "New artist");
    assert_no_mus_files(&music_dir);
    assert!(fs::read_dir(&cache_dir).unwrap().next().is_none());
}

#[tokio::test]
async fn id3_tag_rewrite_standardizes_wav_to_id3v23() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.wav");
    create_wav_with_id3v24(&path);

    standardize_audio_tags(&path, &tmp.path().join("cache"))
        .await
        .unwrap();
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
    standardize_audio_tags(&path, &tmp.path().join("cache"))
        .await
        .unwrap();
    let before = fs::metadata(&path).unwrap().modified().unwrap();

    standardize_audio_tags(&path, &tmp.path().join("cache"))
        .await
        .unwrap();
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

    standardize_audio_tags(&path, &tmp.path().join("cache"))
        .await
        .unwrap();

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

    assert!(extract_cover(&path, &covers_dir, "1").await.unwrap());
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

    assert!(extract_cover(&path, &covers_dir, "1").await.unwrap());
    assert!(covers_dir.join("1_original.webp").is_file());
    assert!(covers_dir.join("1_small.webp").is_file());
}

fn cut_update(cut: AudioCut) -> AudioUpdate<'static> {
    AudioUpdate {
        title: None,
        artist: None,
        cover_jpeg_path: None,
        target_path: None,
        standardize_tags: false,
        update_mtime: true,
        remove_section: Some(cut),
    }
}

fn decoded_samples(path: &Path) -> Vec<u8> {
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args([
            "-map",
            "0:a:0",
            "-f",
            "s16le",
            "-c:a",
            "pcm_s16le",
            "pipe:1",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    output.stdout
}

#[tokio::test]
async fn cut_and_preview_remove_the_exact_samples_without_touching_original_during_preview() {
    let tmp = TempDir::new().unwrap();
    let original = tmp.path().join("original.wav");
    run(Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=997:duration=3:sample_rate=44100",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&original));
    let before = fs::read(&original).unwrap();
    let samples = decoded_samples(&original);

    for (start_seconds, end_seconds) in [(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)] {
        let cut = AudioCut {
            start_seconds,
            end_seconds,
        };
        let path = tmp.path().join("edit.wav");
        fs::copy(&original, &path).unwrap();
        #[cfg(unix)]
        let original_inode = file_id(&path);
        let duration = read_cut_source(&path).await.unwrap().duration;
        let preview = preview_cut(&path, cut, duration, false).await.unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        let preview_path = tmp.path().join("preview.wav");
        fs::write(&preview_path, preview).unwrap();
        let mut expected = samples[..(start_seconds * 44100.0) as usize * 2].to_vec();
        expected.extend_from_slice(&samples[(end_seconds * 44100.0) as usize * 2..]);
        let preview_samples = decoded_samples(&preview_path);
        assert_eq!(preview_samples.len(), expected.len() * 2);
        for (frame, sample) in preview_samples
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<2>().0.iter())
        {
            let left = i16::from_le_bytes([frame[0], frame[1]]) as f64;
            let mono = i16::from_le_bytes([sample[0], sample[1]]) as f64;
            assert!((left - mono * std::f64::consts::FRAC_1_SQRT_2).abs() < 2.0);
        }
        apply_audio_update(&path, &tmp.path().join("cache"), cut_update(cut))
            .await
            .unwrap();
        assert_eq!(decoded_samples(&path), expected);
        #[cfg(unix)]
        assert_eq!(file_id(&path), original_inode);
    }
    assert!(fs::read_dir(tmp.path().join("cache"))
        .unwrap()
        .next()
        .is_none());
}

#[tokio::test]
async fn cut_supported_formats_preserves_tags_cover_and_wav_bit_depth() {
    let tmp = TempDir::new().unwrap();
    for (ext, codec) in [
        ("mp3", "libmp3lame"),
        ("flac", "flac"),
        ("wav", "pcm_s24le"),
        ("m4a", "aac"),
        ("m4a", "alac"),
        ("ogg", "libvorbis"),
    ] {
        let path = tmp.path().join(format!("song.{ext}"));
        let mut command = Command::new("ffmpeg");
        command.args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=997:duration=3",
            "-c:a",
            codec,
            "-metadata",
            "title=Original title",
            "-metadata",
            "artist=Original artist",
        ]);
        command.arg(&path);
        run(&mut command);
        if ext == "mp3" || ext == "wav" {
            let cover = tmp.path().join("cover.jpg");
            create_jpeg(&cover);
            add_id3_cover(&path, &cover);
            let mut tag = Tag::read_from_path(&path).unwrap();
            tag.set_duration(3000);
            tag.write_to_path(&path, Version::Id3v23).unwrap();
        }
        let result = apply_audio_update(
            &path,
            &tmp.path().join("cache"),
            cut_update(AudioCut {
                start_seconds: 1.0,
                end_seconds: 2.0,
            }),
        )
        .await
        .unwrap();
        assert!(result.content_changed);
        let source = read_cut_source(&path).await.unwrap();
        assert!(
            (source.duration - 2.0).abs() < 0.15,
            "{ext}: {}",
            source.duration
        );
        let data = probe(&path);
        let tags = if ext == "ogg" {
            &data["streams"][0]["tags"]
        } else {
            &data["format"]["tags"]
        };
        if ext != "wav" {
            assert_eq!(tags["title"], "Original title", "{ext}");
            assert_eq!(tags["artist"], "Original artist", "{ext}");
        }
        if ext == "mp3" || ext == "wav" {
            assert_has_attached_cover(&data);
            assert!(Tag::read_from_path(&path).unwrap().duration().is_none());
        }
        if ext == "wav" {
            assert_eq!(source.codec, "pcm_s24le");
        }
        if codec == "alac" {
            assert_eq!(source.codec, "alac");
        }

        apply_audio_update(
            &path,
            &tmp.path().join("cache"),
            AudioUpdate {
                title: Some("Edited title"),
                artist: Some("Edited artist"),
                ..cut_update(AudioCut {
                    start_seconds: 0.5,
                    end_seconds: 1.0,
                })
            },
        )
        .await
        .unwrap();
        let edited = probe(&path);
        let edited_tags = if ext == "ogg" {
            &edited["streams"][0]["tags"]
        } else {
            &edited["format"]["tags"]
        };
        assert_eq!(edited_tags["title"], "Edited title", "{ext}");
        assert_eq!(edited_tags["artist"], "Edited artist", "{ext}");
    }
    assert!(fs::read_dir(tmp.path().join("cache"))
        .unwrap()
        .next()
        .is_none());
}

#[tokio::test]
async fn invalid_cuts_leave_the_original_unchanged() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.wav");
    create_wav_with_id3v24(&path);
    let original = fs::read(&path).unwrap();
    for (start_seconds, end_seconds) in [
        (0.0, 0.1),
        (-1.0, 0.05),
        (0.08, 0.02),
        (0.05, 5.0),
        (f64::NAN, 0.05),
        (0.0, f64::INFINITY),
    ] {
        assert!(apply_audio_update(
            &path,
            &tmp.path().join("cache"),
            cut_update(AudioCut {
                start_seconds,
                end_seconds
            })
        )
        .await
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    assert!(!tmp.path().join("cache").exists());
}

#[tokio::test]
async fn failed_publication_restores_original_bytes_identity_and_mtime() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("song.wav");
    create_wav_with_id3v24(&path);
    let before = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    #[cfg(unix)]
    let original_id = file_id(&path);
    let cache = tmp.path().join("cache");
    let destination = tmp.path().join("missing/song.wav");
    let result = apply_audio_update(
        &path,
        &cache,
        AudioUpdate {
            title: Some("New title"),
            artist: None,
            cover_jpeg_path: None,
            target_path: Some(&destination),
            standardize_tags: false,
            update_mtime: true,
            remove_section: None,
        },
    )
    .await;
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    #[cfg(unix)]
    assert_eq!(file_id(&path), original_id);
    assert!(fs::read_dir(&cache).unwrap().next().is_none());
    assert!(!destination.exists());
}
