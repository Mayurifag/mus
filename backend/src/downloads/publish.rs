use std::{fs, path::Path};

use anyhow::Result;
use id3::{
    frame::{Picture, PictureType},
    Tag, TagLike, Version,
};

use crate::media::{apply_audio_update, AudioUpdate};

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

// Only call on the private download staging file, never on a mounted music file.
pub(super) fn prepare_tags(
    path: &Path,
    title: Option<&str>,
    artist: Option<&str>,
    cover: Option<&Path>,
) -> Result<bool> {
    let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());
    let mut changed = tag.version() != Version::Id3v23;
    if let Some(title) = title.filter(|title| Some(*title) != tag.title()) {
        tag.set_title(title);
        changed = true;
    }
    if let Some(artist) = artist.filter(|artist| Some(*artist) != tag.artist()) {
        tag.set_artist(artist);
        changed = true;
    }
    if let Some(cover) = cover {
        tag.remove_all_pictures();
        tag.add_frame(Picture {
            mime_type: "image/jpeg".into(),
            picture_type: PictureType::CoverFront,
            description: String::new(),
            data: fs::read(cover)?,
        });
        changed = true;
    }
    if changed {
        tag.write_to_path(path, Version::Id3v23)?;
    }
    Ok(changed)
}
