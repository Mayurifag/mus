use std::{fs, io, path::Path};

use crate::state::AppState;

pub async fn delete_track_derived_files(state: &AppState, id: &str) {
    let hls = state.data_dir.join(".cache").join("hls").join(id);
    let small = state.covers_dir.join(format!("{id}_small.webp"));
    let original = state.covers_dir.join(format!("{id}_original.webp"));
    let track_id = id.to_owned();
    if let Err(error) = tokio::task::spawn_blocking(move || {
        if let Err(error) = fs::remove_dir_all(hls)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(track_id, error = %error, "failed to delete HLS cache");
        }
        remove_file(&small);
        remove_file(&original);
    })
    .await
    {
        tracing::warn!(track_id = id, error = %error, "derived file cleanup worker failed");
    }
}

fn remove_file(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != io::ErrorKind::NotFound
    {
        tracing::warn!(path = %path.display(), error = %error, "failed to delete derived file");
    }
}
