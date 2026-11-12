//! ID3 inspection and staged rewrites that preserve unreadable source metadata.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use id3::{
    frame::{Picture, PictureType},
    Tag, TagLike, Version,
};
use tempfile::TempPath;

use super::{
    audio_update::AudioUpdate,
    files::{allocate_path, copy_over},
};

pub(super) fn supports_id3(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "aif" | "aiff"
            )
        })
}

pub(crate) fn read_for_update(path: &Path) -> Result<Tag> {
    // Partial tags are not safe to write back: doing so discards unread frames.
    Ok(id3::no_tag_ok(Tag::read_from_path(path))
        .with_context(|| format!("failed to read ID3 tags from {}", path.display()))?
        .unwrap_or_default())
}

pub(super) fn needs_standardization(path: &Path) -> Result<bool> {
    Ok(supports_id3(path) && read_for_update(path)?.version() != Version::Id3v23)
}

pub(super) async fn rewrite_to_temp(
    path: &Path,
    cache_dir: &Path,
    update: &AudioUpdate<'_>,
) -> Result<TempPath> {
    let path = path.to_path_buf();
    let cache_dir = cache_dir.to_path_buf();
    let title = update.title.map(str::to_owned);
    let artist = update.artist.map(str::to_owned);
    let cover = update.cover_jpeg_path.map(Path::to_path_buf);
    tokio::task::spawn_blocking(move || {
        let tmp = allocate_path(&cache_dir, "audio", &path)?;
        copy_over(&path, &tmp)?;
        let mut tag = read_for_update(&tmp)?;
        update_tag(
            &mut tag,
            title.as_deref(),
            artist.as_deref(),
            cover.as_deref(),
        )?;
        tag.write_to_path(&tmp, Version::Id3v23)?;
        Ok(tmp)
    })
    .await?
}

pub(crate) fn update_tag(
    tag: &mut Tag,
    title: Option<&str>,
    artist: Option<&str>,
    cover: Option<&Path>,
) -> Result<()> {
    if let Some(title) = title {
        tag.set_title(title);
    }
    if let Some(artist) = artist {
        tag.set_artist(artist);
    }
    if let Some(cover) = cover {
        tag.remove_all_pictures();
        tag.add_frame(Picture {
            mime_type: "image/jpeg".into(),
            picture_type: PictureType::CoverFront,
            description: String::new(),
            data: fs::read(cover)?,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_not_a_missing_tag() {
        let directory = tempfile::TempDir::new().unwrap();
        let error = needs_standardization(&directory.path().join("absent.mp3")).unwrap_err();
        let io_error = error
            .chain()
            .find_map(|cause| cause.downcast_ref::<std::io::Error>())
            .unwrap();
        assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
    }
}
