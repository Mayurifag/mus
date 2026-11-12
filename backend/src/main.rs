//! Server startup, storage initialization, and background task wiring.

use std::{
    collections::HashMap,
    env, fs, io,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use mus_backend::{
    app::app,
    db::init_db,
    process::command_output_text_with_timeout,
    scanner::{scan_music_dir, watch_music_dir},
    state::AppState,
};
use rusqlite::Connection;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()))
        .init();

    let state = build_state().await?;
    tokio::spawn({
        let state = state.clone();
        async move {
            if let Err(error) = scan_music_dir(state.clone()).await {
                tracing::warn!("failed to scan music directory: {error}");
            }
        }
    });
    tokio::spawn(watch_music_dir(state.clone()));
    let app = app(state);

    let port = env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn build_state() -> Result<AppState> {
    let data_dir = PathBuf::from(env::var("DATA_DIR_PATH").unwrap_or_else(|_| "./app_data".into()));
    let music_dir = data_dir.join("music");
    let cache_dir = data_dir.join(".cache");
    let covers_dir = cache_dir.join("covers");
    let static_dir = env::var("STATIC_DIR_PATH").ok().map(PathBuf::from);
    let static_dir = match static_dir {
        Some(path)
            if tokio::fs::metadata(&path)
                .await
                .is_ok_and(|meta| meta.is_dir()) =>
        {
            Some(path)
        }
        _ => None,
    };
    let database_root = data_dir.clone();
    let conn = tokio::task::spawn_blocking(move || -> Result<Connection> {
        let cache_dir = database_root.join(".cache");
        fs::create_dir_all(&database_root)?;
        fs::create_dir_all(&cache_dir)?;
        migrate_legacy_app_data(&database_root, &cache_dir)?;
        fs::create_dir_all(cache_dir.join("covers"))?;
        let mut conn = Connection::open(cache_dir.join("mus.db"))?;
        init_db(&mut conn)?;
        Ok(conn)
    })
    .await??;

    let (events, _) = broadcast::channel(100);
    Ok(AppState {
        db: Arc::new(Mutex::new(conn)),
        data_dir,
        static_dir,
        music_dir,
        covers_dir,
        events,
        download_lock: Arc::new(tokio::sync::Semaphore::new(1)),
        mutation_locks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        app_date: env::var("BUILD_DATE")
            .ok()
            .and_then(|v| v.get(..10).map(str::to_string))
            .unwrap_or_else(|| "unknown".into()),
        commit_sha: env::var("COMMIT_SHA").ok().filter(|v| !v.is_empty()),
        yt_dlp_version: Arc::new(tokio::sync::Mutex::new(
            command_output_text_with_timeout("yt-dlp", &["--version"], Duration::from_secs(10))
                .await
                .ok(),
        )),
    })
}

fn migrate_legacy_app_data(data_dir: &Path, cache_dir: &Path) -> Result<()> {
    let legacy_db = data_dir.join("mus.db");
    let db = cache_dir.join("mus.db");
    if legacy_db.is_file() && !db.exists() {
        // Merge committed WAL data before separating the database from its sidecars.
        let conn = Connection::open(&legacy_db)?;
        let busy: i64 = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
        anyhow::ensure!(
            busy == 0,
            "Cannot move legacy database while its WAL is in use"
        );
        conn.close().map_err(|(_, error)| error)?;
        move_legacy_database(&legacy_db, &db, cache_dir)?;
    }

    let legacy_covers = data_dir.join("covers");
    let covers_dir = cache_dir.join("covers");
    if legacy_covers.is_dir() && !covers_dir.exists() {
        move_legacy_covers(&legacy_covers, &covers_dir, cache_dir)?;
    }

    Ok(())
}

fn move_legacy_database(source: &Path, destination: &Path, cache_dir: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            let mut staged = tempfile::Builder::new()
                .prefix(".mus-db-migration-")
                .tempfile_in(cache_dir)?;
            io::copy(&mut fs::File::open(source)?, staged.as_file_mut())?;
            staged.as_file().sync_all()?;
            staged.persist_noclobber(destination)?;
            #[cfg(unix)]
            fs::File::open(cache_dir)?.sync_all()?;
            fs::remove_file(source)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn move_legacy_covers(source: &Path, destination: &Path, cache_dir: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            let staged = tempfile::Builder::new()
                .prefix(".mus-covers-migration-")
                .tempdir_in(cache_dir)?;
            copy_legacy_covers(source, staged.path())?;
            fs::rename(staged.path(), destination)?;
            fs::remove_dir_all(source)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn copy_legacy_covers(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        #[cfg(unix)]
        if file_type.is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(entry.path())?, target)?;
            continue;
        }
        if file_type.is_dir() {
            fs::create_dir(&target)?;
            copy_legacy_covers(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_database_move_preserves_committed_wal_data() {
        const CHILD_DATABASE: &str = "MUS_LEGACY_WAL_TEST_DATABASE";
        if let Some(path) = env::var_os(CHILD_DATABASE) {
            let conn = Connection::open(path).unwrap();
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA wal_autocheckpoint = 0;
                 CREATE TABLE committed_data (value TEXT);
                 INSERT INTO committed_data VALUES ('committed before shutdown');",
            )
            .unwrap();
            // Exit without dropping SQLite's connection, as after an abrupt shutdown.
            std::process::exit(0);
        }
        let directory = tempfile::TempDir::new().unwrap();
        let cache = directory.path().join(".cache");
        fs::create_dir(&cache).unwrap();
        let database = directory.path().join("mus.db");
        create_legacy_database(&database);

        migrate_legacy_app_data(directory.path(), &cache).unwrap();

        assert_legacy_data(&cache.join("mus.db"));
        assert!(!database.exists());
    }

    fn create_legacy_database(database: &Path) {
        const CHILD_DATABASE: &str = "MUS_LEGACY_WAL_TEST_DATABASE";
        let status = std::process::Command::new(env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::legacy_database_move_preserves_committed_wal_data",
            ])
            .env(CHILD_DATABASE, database)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(
            fs::metadata(database.with_file_name("mus.db-wal"))
                .unwrap()
                .len()
                > 0
        );
    }

    fn assert_legacy_data(database: &Path) {
        let conn = Connection::open(database).unwrap();
        let value: String = conn
            .query_row("SELECT value FROM committed_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "committed before shutdown");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn legacy_database_and_covers_move_across_filesystems() {
        use std::os::unix::fs::{symlink, MetadataExt};

        let directory = tempfile::TempDir::new().unwrap();
        let cache = tempfile::TempDir::new_in("/dev/shm").unwrap();
        assert_ne!(
            fs::metadata(directory.path()).unwrap().dev(),
            fs::metadata(cache.path()).unwrap().dev()
        );
        symlink(cache.path(), directory.path().join(".cache")).unwrap();
        let database = directory.path().join("mus.db");
        create_legacy_database(&database);
        let covers = directory.path().join("covers");
        fs::create_dir_all(covers.join("nested")).unwrap();
        fs::write(covers.join("nested/cover.webp"), b"legacy cover").unwrap();
        symlink("nested/cover.webp", covers.join("linked.webp")).unwrap();

        migrate_legacy_app_data(directory.path(), &directory.path().join(".cache")).unwrap();

        assert_legacy_data(&cache.path().join("mus.db"));
        assert_eq!(
            fs::read(cache.path().join("covers/nested/cover.webp")).unwrap(),
            b"legacy cover"
        );
        assert_eq!(
            fs::read_link(cache.path().join("covers/linked.webp")).unwrap(),
            Path::new("nested/cover.webp")
        );
        assert!(!database.exists());
        assert!(!covers.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn conflicting_cross_device_cover_move_preserves_both_directories() {
        let directory = tempfile::TempDir::new().unwrap();
        let cache = tempfile::TempDir::new_in("/dev/shm").unwrap();
        let covers = directory.path().join("covers");
        fs::create_dir(&covers).unwrap();
        fs::write(covers.join("legacy.webp"), b"legacy").unwrap();
        let destination = cache.path().join("covers");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("current.webp"), b"current").unwrap();

        assert!(move_legacy_covers(&covers, &destination, cache.path()).is_err());

        assert_eq!(fs::read(covers.join("legacy.webp")).unwrap(), b"legacy");
        assert_eq!(
            fs::read(destination.join("current.webp")).unwrap(),
            b"current"
        );
        assert_eq!(fs::read_dir(cache.path()).unwrap().count(), 1);
    }
}
