//! SQLite operations executed through one private blocking-worker boundary.
//!
//! Async operations acquire the connection inside the worker; transactions do
//! not cross `.await` points. Startup initialization is synchronous.
//! Async database operations require a Tokio runtime.
//!
//! ```no_run
//! use mus_backend::{db, state::AppState};
//! # async fn inspect(state: &AppState) -> anyhow::Result<()> {
//! let tracks = db::list_tracks(state).await?;
//! if let Some(track) = tracks.first() {
//!     let loaded = db::get_track(state, &track.id).await?;
//!     let finalized = db::track_tags_finalized(state, &track.id).await?;
//!     # let _ = (loaded, finalized);
//! }
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::state::AppState;

mod player;
mod schema;
mod tags;
mod tracks;

pub(crate) use player::{load_player_state, save_player_state};
pub use schema::init_db;
pub(crate) use tags::{add_inferred_track_tags, set_track_tags};
pub use tags::{set_download_tags, track_tags_finalized};
pub(crate) use tracks::{
    artist_values, delete_track_row, errored_tracks, list_tracks_by_category, save_track,
    save_track_file_signature, save_track_fingerprint, set_track_cover_state, upsert_scanned_track,
};
pub use tracks::{get_track, list_tracks};

async fn run<T: Send + 'static>(
    state: &AppState,
    operation: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
) -> Result<T> {
    let db = Arc::clone(&state.db);
    tokio::task::spawn_blocking(move || {
        let mut conn = db
            .lock()
            .map_err(|_| anyhow::anyhow!("database connection lock poisoned"))?;
        operation(&mut conn)
    })
    .await
    .context("database worker failed")?
}

#[cfg(test)]
mod tests {
    use super::run;

    #[tokio::test]
    async fn sqlite_runs_on_a_worker_not_the_async_runtime_thread() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = crate::test_support::test_state(
            tmp.path().to_path_buf(),
            rusqlite::Connection::open_in_memory().unwrap(),
        );
        let runtime_thread = std::thread::current().id();
        let (worker_thread, value) = run(&state, |conn| {
            let value: i64 = conn.query_row("SELECT 42", [], |row| row.get(0))?;
            Ok((std::thread::current().id(), value))
        })
        .await
        .unwrap();
        assert_ne!(worker_thread, runtime_thread);
        assert_eq!(value, 42);
    }
}
