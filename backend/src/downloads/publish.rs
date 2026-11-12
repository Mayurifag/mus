use std::path::Path;

use anyhow::Result;
use id3::{TagLike, Version};

use crate::media::{
    apply_audio_update,
    id3_tags::{read_for_update, update_tag},
    AudioUpdate,
};

pub(super) async fn publish_download(
    downloaded: &Path,
    cache_dir: &Path,
    final_path: &Path,
    title: Option<&str>,
    artist: Option<&str>,
    cover: Option<&Path>,
) -> Result<()> {
    let path = downloaded.to_path_buf();
    let title = title.map(str::to_string);
    let artist = artist.map(str::to_string);
    let cover = cover.map(Path::to_path_buf);
    tokio::task::spawn_blocking(move || {
        prepare_tags(&path, title.as_deref(), artist.as_deref(), cover.as_deref())
    })
    .await??;
    apply_audio_update(
        downloaded,
        cache_dir,
        AudioUpdate {
            remove_section: None,
            title: None,
            artist: None,
            cover_jpeg_path: None,
            target_path: Some(final_path),
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await?;
    Ok(())
}

pub(super) fn prepare_tags(
    path: &Path,
    title: Option<&str>,
    artist: Option<&str>,
    cover: Option<&Path>,
) -> Result<bool> {
    let mut tag = read_for_update(path)?;
    let title = title.filter(|title| Some(*title) != tag.title());
    let artist = artist.filter(|artist| Some(*artist) != tag.artist());
    let changed =
        tag.version() != Version::Id3v23 || title.is_some() || artist.is_some() || cover.is_some();
    if changed {
        update_tag(&mut tag, title, artist, cover)?;
        tag.write_to_path(path, Version::Id3v23)?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use id3::Tag;
    use std::fs;

    #[tokio::test]
    async fn publication_rejects_unreadable_tags_without_changing_download() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("download.mp3");
        fs::write(&path, b"audio payload").unwrap();
        let mut tag = Tag::new();
        tag.set_title("Original title");
        tag.set_album("Album to preserve");
        tag.write_to_path(&path, Version::Id3v23).unwrap();
        let mut original = fs::read(&path).unwrap();
        let title = original
            .windows(4)
            .position(|bytes| bytes == b"TIT2")
            .unwrap();
        original[title + 10] = 0xff;
        fs::write(&path, &original).unwrap();
        let destination = directory.path().join("published.mp3");

        let error = publish_download(
            &path,
            &directory.path().join("cache"),
            &destination,
            Some("New title"),
            None,
            None,
        )
        .await
        .unwrap_err();
        assert!(error.downcast_ref::<id3::Error>().is_some(), "{error:#}");
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!destination.exists());
    }
}
