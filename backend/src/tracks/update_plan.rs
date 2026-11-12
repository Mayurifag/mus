use std::path::{Path, PathBuf};

use crate::{
    audio_names::{generate_filename, normalize_artists, normalize_text},
    categories::inferred_tag_names,
    db,
    error::AppError,
    files::path_exists_as_different_file,
    models::{Track, TrackUpdate},
    state::AppState,
};

/// Decisions shared by request validation and the worker's fresh, locked read.
pub(super) struct UpdatePlan {
    pub title: String,
    pub artist: String,
    pub path: PathBuf,
    pub tags: Vec<String>,
    pub metadata_changed: bool,
    pub tags_changed: bool,
    pub finalize_tags: bool,
    pub rename_file: bool,
}

impl UpdatePlan {
    pub async fn new(
        state: &AppState,
        track: &Track,
        update: &TrackUpdate,
    ) -> Result<Self, AppError> {
        let title = normalize_text(update.title.as_deref().unwrap_or(&track.title));
        let artist = normalize_artists(update.artist.as_deref().unwrap_or(&track.artist));
        let metadata_changed = title != track.title || artist != track.artist;
        let rename_file = update.rename_file.unwrap_or(false);
        let mut path = PathBuf::from(&track.file_path);
        if rename_file {
            let ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("mp3");
            let filename = generate_filename(&artist, &title, ext)?;
            path = path.parent().unwrap_or(&state.music_dir).join(filename);
            if path != Path::new(&track.file_path) {
                let source = PathBuf::from(&track.file_path);
                let destination = path.clone();
                let conflict = tokio::task::spawn_blocking(move || {
                    path_exists_as_different_file(&source, &destination)
                })
                .await
                .map_err(anyhow::Error::from)??;
                if conflict {
                    return Err(AppError::conflict("A file with this name already exists"));
                }
            }
        }

        let old_tags = track
            .tags
            .iter()
            .map(|tag| tag.name.clone())
            .collect::<Vec<_>>();
        let finalized = db::track_tags_finalized(state, &track.id).await?;
        let tags = update.tags.clone().unwrap_or_else(|| {
            if finalized || (update.remove_section.is_some() && !metadata_changed) {
                old_tags.clone()
            } else {
                inferred_tag_names(&title)
            }
        });
        Ok(Self {
            title,
            artist,
            path,
            tags_changed: tags != old_tags || (update.tags.is_some() && !finalized),
            tags,
            metadata_changed,
            finalize_tags: update.tags.is_some(),
            rename_file,
        })
    }

    pub fn changes_audio(&self, update: &TrackUpdate) -> bool {
        self.metadata_changed
            || self.rename_file
            || update
                .artwork_url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty())
            || update.remove_section.is_some()
    }
}
