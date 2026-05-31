use std::{fs, io};

use crate::{hls::delete_hls_cache, state::AppState};

pub fn delete_track_derived_files(state: &AppState, id: &str) {
    delete_hls_cache(state, id);
    remove_file(state.covers_dir.join(format!("{id}_small.webp")));
    remove_file(state.covers_dir.join(format!("{id}_original.webp")));
}

fn remove_file(path: std::path::PathBuf) {
    if let Err(error) = fs::remove_file(&path) {
        if error.kind() != io::ErrorKind::NotFound {
            tracing::warn!(path = %path.display(), error = %error, "failed to delete derived file");
        }
    }
}
